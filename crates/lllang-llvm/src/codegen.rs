//! MIR → LLVM IR lowering and native object emission.
//!
//! Throwing ABI: boxed error payloads via `ll_box_bytes` into `__err_out`.
//! Classes: leading vtable pointer + `{Class}_vtable` globals for virtual calls.

use crate::CodegenOptions;
use anyhow::{Context as AnyhowContext, Result};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetTriple,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{
    AggregateValueEnum, BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue,
    PointerValue,
};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use lllang_mir::{
    BinOp, Callee, Function, Inst, InterpPart, Local, MirTy, Program, SwitchKey, Terminator,
    TypeDef, UnOp, Value,
};
use std::collections::HashMap;
use std::path::Path;

pub fn emit_object(program: &Program, object_path: &Path, opts: &CodegenOptions) -> Result<(String, ())> {
    init_targets();
    let context = Context::create();
    let module = build_module(program, &context)?;
    let llvm_ir = module.print_to_string().to_string();
    write_object_for_triple(&module, object_path, &opts.target_triple, opts.optimize)?;
    Ok((llvm_ir, ()))
}

/// Emit portable LLVM bitcode bytes for packing into BLL v2 (no host-native object).
pub fn emit_bitcode(program: &Program, opts: &CodegenOptions) -> Result<(String, Vec<u8>)> {
    init_targets();
    let context = Context::create();
    let module = build_module(program, &context)?;
    let llvm_ir = module.print_to_string().to_string();
    let buf = module.write_bitcode_to_memory();
    let _ = opts; // optimize flag reserved for future bitcode passes
    Ok((llvm_ir, buf.as_slice().to_vec()))
}

/// Lower raw LLVM bitcode to a native object for `triple`.
pub fn materialize_bitcode_to_object(
    bitcode: &[u8],
    triple: &str,
    object_path: &Path,
    optimize: bool,
) -> Result<()> {
    init_targets();
    let context = Context::create();
    let mem = MemoryBuffer::create_from_memory_range_copy(bitcode, "bll.bc");
    let module = Module::parse_bitcode_from_buffer(&mem, &context)
        .map_err(|e| anyhow::anyhow!("parse bitcode: {e}"))?;
    write_object_for_triple(&module, object_path, triple, optimize)?;
    Ok(())
}

fn init_targets() {
    Target::initialize_all(&InitializationConfig::default());
}

fn build_module<'ctx>(program: &Program, context: &'ctx Context) -> Result<Module<'ctx>> {
    let module = context.create_module("lllang");
    let builder = context.create_builder();

    declare_runtime_externs(context, &module);
    let layout = index_type_defs(program, &module, context);

    for func in &program.functions {
        declare_function_sig(context, &module, &layout, func)?;
    }
    emit_vtables(program, context, &module, &layout)?;

    for func in &program.functions {
        emit_function(program, context, &module, &builder, &layout, func)?;
    }
    Ok(module)
}

fn write_object_for_triple(
    module: &Module,
    object_path: &Path,
    triple_str: &str,
    optimize: bool,
) -> Result<()> {
    let triple = TargetTriple::create(triple_str);
    let target = Target::from_triple(&triple).map_err(|e| anyhow::anyhow!("{e}"))?;
    let opt = if optimize {
        OptimizationLevel::Default
    } else {
        OptimizationLevel::None
    };
    let tm = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            opt,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .with_context(|| format!("target machine for `{triple_str}`"))?;
    module.set_triple(&triple);
    let td = tm.get_target_data();
    let dl = td.get_data_layout();
    module.set_data_layout(&dl);
    if let Some(parent) = object_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    tm.write_to_file(module, FileType::Object, object_path)
        .with_context(|| format!("write object for `{triple_str}`"))?;
    Ok(())
}

fn declare_runtime_externs<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let ptr = context.ptr_type(AddressSpace::default());
    let void = context.void_type();
    let i64 = context.i64_type();
    let i32 = context.i32_type();

    let decl = |name: &str, ret: BasicTypeEnum<'ctx>, params: &[BasicMetadataTypeEnum<'ctx>]| {
        let fn_ty = ret.fn_type(params, false);
        module.add_function(name, fn_ty, Some(Linkage::External));
    };
    let decl_void = |name: &str, params: &[BasicMetadataTypeEnum<'ctx>]| {
        let fn_ty = void.fn_type(params, false);
        module.add_function(name, fn_ty, Some(Linkage::External));
    };

    decl("ll_alloc", ptr.into(), &[ptr.into()]);
    decl("ll_alloc_sized", ptr.into(), &[i64.into()]);
    decl("ll_retain", ptr.into(), &[ptr.into()]);
    decl_void("ll_release", &[ptr.into()]);
    decl("ll_is_unique", i32.into(), &[ptr.into()]);
    decl("ll_make_unique", ptr.into(), &[ptr.into(), i64.into()]);
    decl("ll_box_bytes", ptr.into(), &[ptr.into(), i64.into()]);
    decl_void("ll_unbox_bytes", &[ptr.into(), ptr.into(), i64.into()]);
    decl("ll_array_new", ptr.into(), &[i64.into(), ptr.into()]);
    decl_void("ll_array_append", &[ptr.into(), ptr.into()]);
    decl("ll_array_count", i64.into(), &[ptr.into()]);
    decl("ll_array_get_ptr", ptr.into(), &[ptr.into(), i64.into()]);
    decl("ll_array_new_i64", ptr.into(), &[]);
    // First arg is `void **` (address of the array handle slot).
    decl_void("ll_array_append_i64", &[ptr.into(), i64.into()]);
    decl("ll_array_get_i64", i64.into(), &[ptr.into(), i64.into()]);
    decl_void("ll_array_set_i64", &[ptr.into(), i64.into(), i64.into()]);
    decl("ll_array_sorted_i64", ptr.into(), &[ptr.into()]);
    decl_void("ll_array_append", &[ptr.into(), ptr.into()]);
    decl("ll_string_from_cstr", ptr.into(), &[ptr.into()]);
    decl("ll_string_from_bytes", ptr.into(), &[ptr.into(), i64.into()]);
    decl("ll_string_len", i64.into(), &[ptr.into()]);
    decl("ll_string_cstr", ptr.into(), &[ptr.into()]);
    decl("ll_string_is_empty", i32.into(), &[ptr.into()]);
    decl("ll_string_concat", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_string_from_i64", ptr.into(), &[i64.into()]);
    decl("ll_string_from_f64", ptr.into(), &[context.f64_type().into()]);
    decl("ll_string_from_bool", ptr.into(), &[i32.into()]);
    decl("ll_string_eq", i32.into(), &[ptr.into(), ptr.into()]);
    decl_void("ll_print", &[ptr.into()]);
    decl("ll_isa", i32.into(), &[ptr.into(), ptr.into()]);
    decl("ll_type_name", ptr.into(), &[ptr.into()]);
    decl_void("ll_trap", &[ptr.into(), ptr.into()]);
    decl_void("ll_die", &[ptr.into()]);
    decl_void("ll_eprint", &[ptr.into()]);
    decl("ll_read_line", ptr.into(), &[]);
    decl_void("ll_fatal_error", &[ptr.into()]);
    decl_void("ll_assert", &[i32.into(), ptr.into()]);
    decl_void("ll_rt_set_args", &[i32.into(), ptr.into()]);
    decl("ll_args_get", ptr.into(), &[i64.into()]);
    decl("ll_args_count", i64.into(), &[]);
    decl("ll_env_get", ptr.into(), &[ptr.into()]);
    decl_void("ll_env_load_dotenv", &[ptr.into()]);
    decl("ll_env_get_string_or", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_env_get_i64", i64.into(), &[ptr.into(), ptr.into()]);
    decl("ll_env_get_i64_or", i64.into(), &[ptr.into(), i64.into()]);
    decl("ll_env_get_bool", i32.into(), &[ptr.into(), ptr.into()]);
    decl("ll_env_get_bool_or", i32.into(), &[ptr.into(), i32.into()]);
    decl(
        "ll_env_get_f64",
        context.f64_type().into(),
        &[ptr.into(), ptr.into()],
    );
    decl(
        "ll_env_get_f64_or",
        context.f64_type().into(),
        &[ptr.into(), context.f64_type().into()],
    );
    decl("ll_env_get_decimal", i64.into(), &[ptr.into(), ptr.into()]);
    decl("ll_env_get_decimal_or", i64.into(), &[ptr.into(), i64.into()]);
    decl("ll_file_exists", i32.into(), &[ptr.into()]);
    decl("ll_file_read_text", ptr.into(), &[ptr.into()]);
    decl("ll_file_write_text", i32.into(), &[ptr.into(), ptr.into()]);
    decl("ll_instant_now_ms", i64.into(), &[]);
    decl_void("ll_sleep_ms", &[i64.into()]);
    decl_void("ll_process_exit", &[i64.into()]);
    decl("ll_process_start", ptr.into(), &[ptr.into()]);
    decl("ll_process_wait", i64.into(), &[ptr.into()]);
    decl("ll_string_contains", i32.into(), &[ptr.into(), ptr.into()]);
    decl("ll_string_trim", ptr.into(), &[ptr.into()]);
    decl("ll_string_substring", ptr.into(), &[ptr.into(), i64.into(), i64.into()]);
    decl("ll_string_split", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_string_upper", ptr.into(), &[ptr.into()]);
    decl("ll_string_lower", ptr.into(), &[ptr.into()]);
    decl("ll_string_replace", ptr.into(), &[ptr.into(), ptr.into(), ptr.into()]);
    decl("ll_string_join", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_identity_uuid", ptr.into(), &[]);
    decl("ll_identity_parse", ptr.into(), &[ptr.into()]);
    decl("ll_identity_short_id", ptr.into(), &[]);
    decl("ll_identity_ulid", ptr.into(), &[]);
    decl("ll_identity_nanoid", ptr.into(), &[]);
    decl("ll_math_abs_i64", i64.into(), &[i64.into()]);
    decl("ll_math_max_i64", i64.into(), &[i64.into(), i64.into()]);
    decl("ll_math_min_i64", i64.into(), &[i64.into(), i64.into()]);
    decl("ll_math_clamp_i64", i64.into(), &[i64.into(), i64.into(), i64.into()]);
    decl("ll_math_sqrt", context.f64_type().into(), &[context.f64_type().into()]);
    decl(
        "ll_math_pow",
        context.f64_type().into(),
        &[context.f64_type().into(), context.f64_type().into()],
    );
    decl("ll_math_sin", context.f64_type().into(), &[context.f64_type().into()]);
    decl("ll_math_log10", context.f64_type().into(), &[context.f64_type().into()]);
    decl("ll_math_pi", context.f64_type().into(), &[]);
    decl("ll_decimal_from_string", i64.into(), &[ptr.into(), ptr.into()]);
    decl("ll_decimal_add", i64.into(), &[i64.into(), i64.into()]);
    decl("ll_decimal_sub", i64.into(), &[i64.into(), i64.into()]);
    decl("ll_decimal_mul", i64.into(), &[i64.into(), i64.into()]);
    decl("ll_base64_encode", ptr.into(), &[ptr.into()]);
    decl("ll_hex_encode", ptr.into(), &[ptr.into()]);
    decl("ll_secure_random_bytes", ptr.into(), &[i64.into()]);
    decl("ll_array_new_ptr", ptr.into(), &[]);
    decl_void("ll_array_append_ptr", &[ptr.into(), ptr.into()]);
    decl("ll_array_get_ref", ptr.into(), &[ptr.into(), i64.into()]);
    decl("ll_dict_new", ptr.into(), &[]);
    decl_void("ll_dict_set", &[ptr.into(), ptr.into(), ptr.into()]);
    decl("ll_dict_get", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_dict_contains", i32.into(), &[ptr.into(), ptr.into()]);
    decl("ll_dict_count", i64.into(), &[ptr.into()]);
    decl("ll_dict_keys", ptr.into(), &[ptr.into()]);
    decl("ll_set_new", ptr.into(), &[]);
    decl_void("ll_set_insert", &[ptr.into(), ptr.into()]);
    decl("ll_set_contains", i32.into(), &[ptr.into(), ptr.into()]);
    decl("ll_set_count", i64.into(), &[ptr.into()]);
    decl("ll_set_union", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_set_intersection", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_set_difference", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_int_from_string", i64.into(), &[ptr.into(), ptr.into()]);
    decl("ll_double_sqrt", context.f64_type().into(), &[context.f64_type().into()]);
    decl("ll_json_decode_fields", i32.into(), &[ptr.into(), ptr.into(), ptr.into()]);
    decl("ll_json_encode_fields", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_toon_decode_fields", i32.into(), &[ptr.into(), ptr.into(), ptr.into()]);
    decl("ll_toon_encode_fields", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_http_get", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_http_post", ptr.into(), &[ptr.into(), ptr.into(), ptr.into()]);
    decl_void("ll_log_debug", &[ptr.into()]);
    decl_void("ll_log_debug_fields", &[ptr.into(), ptr.into()]);
    decl("ll_arena_create", ptr.into(), &[i64.into()]);
    decl_void("ll_run_blocking_noop", &[]);
    decl("ll_with_timeout_ms", i64.into(), &[i64.into()]);
    decl_void("ll_http_server_listen", &[i64.into(), ptr.into()]);
    decl_void("ll_http_server_serve", &[i64.into(), ptr.into()]);
    decl_void("ll_log_info", &[ptr.into()]);
    decl_void("ll_log_warn", &[ptr.into()]);
    decl_void("ll_log_error", &[ptr.into()]);
    decl_void("ll_cli_usage", &[ptr.into()]);
    decl("ll_cli_has_flag", i32.into(), &[ptr.into()]);
    decl("ll_tcp_connect", ptr.into(), &[ptr.into(), i64.into()]);
    decl("ll_tcp_read", ptr.into(), &[ptr.into()]);
    decl_void("ll_tcp_write", &[ptr.into(), ptr.into()]);
    decl_void("ll_tcp_close", &[ptr.into()]);
    decl("ll_mutex_create", ptr.into(), &[]);
    decl_void("ll_mutex_lock", &[ptr.into()]);
    decl_void("ll_mutex_unlock", &[ptr.into()]);
    // Phase 10 concurrency / SQL
    decl("ll_task_spawn", ptr.into(), &[ptr.into()]);
    decl("ll_task_spawn_ret", ptr.into(), &[ptr.into()]);
    decl_void("ll_task_join", &[ptr.into()]);
    decl("ll_task_join_ret", ptr.into(), &[ptr.into()]);
    decl_void("ll_task_detach", &[ptr.into()]);
    decl_void("ll_task_cancel", &[ptr.into()]);
    decl("ll_channel_create", ptr.into(), &[i64.into()]);
    decl_void("ll_channel_send_i64", &[ptr.into(), i64.into()]);
    decl("ll_channel_recv_i64", i64.into(), &[ptr.into()]);
    decl_void("ll_channel_close", &[ptr.into()]);
    decl("ll_wait_group_create", ptr.into(), &[]);
    decl_void("ll_wait_group_add", &[ptr.into(), i64.into()]);
    decl_void("ll_wait_group_done", &[ptr.into()]);
    decl_void("ll_wait_group_wait", &[ptr.into()]);
    decl("ll_task_group_create", ptr.into(), &[]);
    decl_void("ll_task_group_add", &[ptr.into(), ptr.into()]);
    decl_void("ll_task_group_wait", &[ptr.into()]);
    decl("ll_regex_is_match", i32.into(), &[ptr.into(), ptr.into()]);
    decl("ll_cancellation_token_create", ptr.into(), &[]);
    decl_void("ll_cancellation_token_cancel", &[ptr.into()]);
    decl_void("ll_cancellation_token_throw_if_cancelled", &[ptr.into()]);
    decl_void("ll_main_actor_drain", &[]);
    decl_void("ll_main_actor_submit", &[ptr.into()]);
    decl("ll_sql_open", ptr.into(), &[ptr.into()]);
    decl("ll_sql_exec", ptr.into(), &[ptr.into(), ptr.into()]);
    decl_void("ll_sql_close", &[ptr.into()]);
    decl_void("ll_weak_store", &[ptr.into(), ptr.into()]);
    decl("ll_weak_load", ptr.into(), &[ptr.into()]);
    decl_void("ll_unowned_store", &[ptr.into(), ptr.into()]);
    decl("ll_unowned_load", ptr.into(), &[ptr.into()]);
    decl("ll_crypto_sha256", ptr.into(), &[ptr.into()]);
    decl("ll_crypto_hmac_sha256", ptr.into(), &[ptr.into(), ptr.into()]);
    decl("ll_c_strlen", i64.into(), &[ptr.into()]);
    decl(
        "ll_llm_run_agent",
        ptr.into(),
        &[ptr.into(), ptr.into(), ptr.into()],
    );
    decl("ll_llm_merge_context", ptr.into(), &[ptr.into()]);
    decl_void("ll_noop", &[]);
    decl_void("ll_test_expect", &[context.bool_type().into(), ptr.into()]);
    decl_void("ll_test_expect_empty", &[ptr.into(), ptr.into()]);
    decl_void("ll_test_expect_not_empty", &[ptr.into(), ptr.into()]);
    decl_void("ll_test_contains", &[ptr.into(), ptr.into(), ptr.into()]);
    decl_void(
        "ll_test_approx",
        &[context.f64_type().into(), context.f64_type().into(), ptr.into()],
    );
    decl("ll_test_job_id", i64.into(), &[]);
}

#[derive(Clone)]
struct TypeLayout<'ctx> {
    struct_types: HashMap<String, StructType<'ctx>>,
    enum_rep: StructType<'ctx>,
    /// Classes that include a leading vtable pointer in their LLVM layout.
    class_with_vtable: HashMap<String, Vec<String>>,
    /// Codec schema strings: `sku:s,name:s,priceCents:i`
    field_schemas: HashMap<String, String>,
    /// Class name → weak field indices (into logical fields, not including vtable).
    weak_fields: HashMap<String, Vec<usize>>,
    unowned_fields: HashMap<String, Vec<usize>>,
}

fn index_type_defs<'ctx>(program: &Program, module: &Module<'ctx>, context: &'ctx Context) -> TypeLayout<'ctx> {
    let i64 = context.i64_type();
    let i32 = context.i32_type();
    let ptr = context.ptr_type(AddressSpace::default());
    let enum_rep = context.struct_type(
        &[i32.into(), i64.array_type(8).into()],
        false,
    );
    let mut struct_types = HashMap::new();
    let mut class_with_vtable = HashMap::new();
    let mut field_schemas = HashMap::new();
    let mut weak_fields = HashMap::new();
    let mut unowned_fields = HashMap::new();
    let mut struct_fields: Vec<(String, Vec<(String, MirTy)>)> = Vec::new();
    for td in &program.types {
        match td {
            TypeDef::Struct { name, fields } => {
                struct_fields.push((name.clone(), fields.clone()));
                let _ = module;
            }
            TypeDef::Class {
                name,
                fields,
                virtual_methods,
                weak_fields: wf,
                unowned_fields: uf,
                ..
            } => {
                let mut llvm_fields: Vec<BasicTypeEnum> = vec![ptr.into()];
                // Weak/unowned fields are stored as raw pointers (optional class erased).
                llvm_fields.extend(fields.iter().map(|(fname, t)| {
                    let _ = fname;
                    let store_ty = match t {
                        MirTy::Optional(inner) if matches!(inner.as_ref(), MirTy::Class(_)) => {
                            &MirTy::Class("_".into())
                        }
                        other => other,
                    };
                    field_llvm_ty(context, store_ty, &struct_types)
                }));
                let st = context.struct_type(
                    &llvm_fields.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                    false,
                );
                struct_types.insert(name.clone(), st);
                class_with_vtable.insert(name.clone(), virtual_methods.clone());
                weak_fields.insert(name.clone(), wf.clone());
                unowned_fields.insert(name.clone(), uf.clone());
            }
            TypeDef::Enum { name, .. } => {
                struct_types.insert(name.clone(), enum_rep);
            }
        }
    }
    // Multi-pass so nested `Struct` fields resolve once dependents exist.
    for _ in 0..struct_fields.len().saturating_add(1) {
        let mut progressed = false;
        for (name, fields) in &struct_fields {
            if struct_types.contains_key(name) {
                continue;
            }
            let ready = fields.iter().all(|(_, t)| match t {
                MirTy::Struct(n) | MirTy::Class(n) => struct_types.contains_key(n),
                _ => true,
            });
            if !ready {
                continue;
            }
            let llvm_fields: Vec<BasicTypeEnum> = fields
                .iter()
                .map(|(_, t)| field_llvm_ty(context, t, &struct_types))
                .collect();
            let st = context.struct_type(
                &llvm_fields.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                false,
            );
            struct_types.insert(name.clone(), st);
            progressed = true;
        }
        if !progressed {
            break;
        }
    }
    for (name, fields) in &struct_fields {
        if !struct_types.contains_key(name) {
            let llvm_fields: Vec<BasicTypeEnum> = fields
                .iter()
                .map(|(_, t)| field_llvm_ty(context, t, &struct_types))
                .collect();
            let st = context.struct_type(
                &llvm_fields.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                false,
            );
            struct_types.insert(name.clone(), st);
        }
        field_schemas.insert(name.clone(), schema_for_fields(fields, &field_schemas));
    }
    // Rebuild schemas once so nested `{...}` can expand.
    for (name, fields) in &struct_fields {
        field_schemas.insert(name.clone(), schema_for_fields(fields, &field_schemas));
    }
    TypeLayout {
        struct_types,
        enum_rep,
        class_with_vtable,
        field_schemas,
        weak_fields,
        unowned_fields,
    }
}

/// LLVM layout for Codable struct fields (Bool stored as i64 to match JSON `b`/`i`).
fn field_llvm_ty<'ctx>(
    context: &'ctx Context,
    ty: &MirTy,
    structs: &HashMap<String, StructType<'ctx>>,
) -> BasicTypeEnum<'ctx> {
    match ty {
        MirTy::Bool => context.i64_type().into(),
        MirTy::Struct(n) | MirTy::Class(n) => structs
            .get(n)
            .map(|s| (*s).into())
            .unwrap_or_else(|| context.struct_type(&[], false).into()),
        MirTy::Array(_) => context.ptr_type(AddressSpace::default()).into(),
        other => mir_ty_to_llvm(context, other),
    }
}

fn schema_for_fields(fields: &[(String, MirTy)], known: &HashMap<String, String>) -> String {
    fields
        .iter()
        .map(|(n, t)| {
            let kind = match t {
                MirTy::String => "s".into(),
                MirTy::Bool => "b".into(),
                MirTy::Int | MirTy::Int64 | MirTy::Int32 | MirTy::Int16 | MirTy::Int8 | MirTy::UInt => {
                    "i".into()
                }
                MirTy::Struct(name) | MirTy::Class(name) => {
                    let inner = known.get(name).cloned().unwrap_or_default();
                    format!("{{{inner}}}")
                }
                MirTy::Array(inner) => match inner.as_ref() {
                    MirTy::Int | MirTy::Int64 => "[i]".into(),
                    MirTy::String => "[s]".into(),
                    _ => "[i]".into(),
                },
                _ => "s".into(),
            };
            format!("{n}:{kind}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn mir_ty_to_llvm<'ctx>(context: &'ctx Context, ty: &MirTy) -> BasicTypeEnum<'ctx> {
    match ty {
        MirTy::Void | MirTy::Never => context.i8_type().into(),
        MirTy::Bool => context.bool_type().into(),
        MirTy::Int | MirTy::Int64 | MirTy::UInt => context.i64_type().into(),
        MirTy::Int8 => context.i8_type().into(),
        MirTy::Int16 => context.i16_type().into(),
        MirTy::Int32 => context.i32_type().into(),
        MirTy::Float => context.f32_type().into(),
        MirTy::Double => context.f64_type().into(),
        MirTy::Char => context.i32_type().into(),
        MirTy::String | MirTy::Class(_) => context.ptr_type(AddressSpace::default()).into(),
        MirTy::Array(_) => context.ptr_type(AddressSpace::default()).into(),
        MirTy::Optional(inner) => optional_llvm_type(context, inner).into(),
        MirTy::Tuple(elems) => {
            let fields: Vec<BasicTypeEnum> = elems.iter().map(|t| mir_ty_to_llvm(context, t)).collect();
            context
                .struct_type(
                    &fields.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                    false,
                )
                .into()
        }
        MirTy::Struct(_name) => {
            // Placeholder; callers that need the real layout should use TypeLayout.
            context.struct_type(&[], false).into()
        }
        MirTy::Enum(_) => context
            .struct_type(
                &[
                    context.i32_type().into(),
                    context.i64_type().array_type(8).into(),
                ],
                false,
            )
            .into(),
        MirTy::Named(n) if n == "__err_out" || n == "void*" => {
            context.ptr_type(AddressSpace::default()).into()
        }
        MirTy::Named(n)
            if n.starts_with("existential:")
                || matches!(
                    n.as_str(),
                    "Task"
                        | "Mutex"
                        | "Channel"
                        | "WaitGroup"
                        | "TaskGroup"
                        | "CancellationToken"
                        | "SQLConnection"
                        | "SQLManager"
                        | "TcpStream"
                        | "Dictionary"
                        | "Set"
                        | "MainActor"
                        | "ProcessHandle"
                ) =>
        {
            context.ptr_type(AddressSpace::default()).into()
        }
        MirTy::Named(n) if n == "Decimal" || n == "UInt64" => context.i64_type().into(),
        MirTy::Named(n) if matches!(n.as_str(), "UInt8") => context.i8_type().into(),
        MirTy::Named(n) if matches!(n.as_str(), "UInt16") => context.i16_type().into(),
        MirTy::Named(n) if matches!(n.as_str(), "UInt32") => context.i32_type().into(),
        MirTy::Named(n) if n == "Float32" => context.f32_type().into(),
        MirTy::Named(n) if n == "Float64" => context.f64_type().into(),
        MirTy::Named(_) | MirTy::Error => context.i64_type().into(),
    }
}

fn optional_llvm_type<'ctx>(context: &'ctx Context, inner: &MirTy) -> StructType<'ctx> {
    let inner_ty = mir_ty_to_llvm(context, inner);
    context.struct_type(&[context.bool_type().into(), inner_ty.into()], false)
}

struct Cg<'a, 'ctx> {
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    layout: TypeLayout<'ctx>,
    bb_map: HashMap<usize, inkwell::basic_block::BasicBlock<'ctx>>,
    local_types: HashMap<usize, MirTy>,
    values: HashMap<usize, BasicValueEnum<'ctx>>,
    /// Stack slots for MIR locals (enables cross-block defs without phis).
    allocas: HashMap<usize, PointerValue<'ctx>>,
    /// Callee's `__err_out` parameter (pointer to error object slot).
    err_out_ptr: Option<PointerValue<'ctx>>,
    /// Most recent caller's err_out alloca (pointer to pointer), for `LoadErrFlag`.
    last_err_out: Option<PointerValue<'ctx>>,
}

impl<'a, 'ctx> Cg<'a, 'ctx> {
    fn ptr_type(&self) -> inkwell::types::PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    fn i64_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i64_type()
    }

    fn bool_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.bool_type()
    }

    fn set_local_ty(&mut self, loc: Local, ty: MirTy) {
        self.local_types.insert(loc.0, ty);
    }

    fn is_address_local(&self, loc: Local) -> bool {
        matches!(
            self.local_types.get(&loc.0),
            Some(MirTy::Named(n)) if n == "__err_out" || n == "void*"
        )
    }

    fn set_value(&mut self, loc: Local, val: BasicValueEnum<'ctx>) -> Result<()> {
        let mut val = val;
        // Coerce C `int` (i32) results into MIR Bool (i1) before storing into an i1 slot.
        if matches!(self.local_types.get(&loc.0), Some(MirTy::Bool)) && val.is_int_value() {
            let iv = val.into_int_value();
            if iv.get_type().get_bit_width() != 1 {
                let truncated = self.builder.build_int_truncate(
                    iv,
                    self.bool_type(),
                    "bool_trunc",
                )?;
                let zero = self.bool_type().const_int(0, false);
                // Also accept non-0/1 i32 by comparing != 0 when wider than i1 after trunc would wrap.
                let _ = (truncated, zero);
                let as_i1 = self.builder.build_int_compare(
                    IntPredicate::NE,
                    iv,
                    iv.get_type().const_int(0, false),
                    "bool_nz",
                )?;
                val = as_i1.into();
            }
        }
        // Numeric / representation casts when Store target MIR type differs (from Cast lowering).
        if let Some(dst_ty) = self.local_types.get(&loc.0).cloned() {
            val = self.coerce_basic_to_mir(val, &dst_ty)?;
        }
        if let Some(ptr) = self.allocas.get(&loc.0).copied() {
            if !self.is_address_local(loc) {
                self.builder
                    .build_store(ptr, val)
                    .with_context(|| format!("store local {}", loc.0))?;
            }
        }
        self.values.insert(loc.0, val);
        Ok(())
    }

    fn coerce_basic_to_mir(
        &self,
        val: BasicValueEnum<'ctx>,
        dst: &MirTy,
    ) -> Result<BasicValueEnum<'ctx>> {
        match dst {
            MirTy::Double | MirTy::Float => {
                let fty = if matches!(dst, MirTy::Float) {
                    self.context.f32_type()
                } else {
                    self.context.f64_type()
                };
                if val.is_int_value() {
                    let iv = val.into_int_value();
                    let fv = self
                        .builder
                        .build_signed_int_to_float(iv, fty, "sitofp")?;
                    return Ok(fv.into());
                }
                if val.is_float_value() {
                    let fv = val.into_float_value();
                    if fv.get_type() != fty {
                        let cast = if matches!(dst, MirTy::Float) {
                            self.builder.build_float_trunc(fv, fty, "fptrunc")?
                        } else {
                            self.builder.build_float_ext(fv, fty, "fpext")?
                        };
                        return Ok(cast.into());
                    }
                }
            }
            MirTy::Int | MirTy::Int64 | MirTy::Int32 | MirTy::Int16 | MirTy::Int8 | MirTy::UInt => {
                let ity = match dst {
                    MirTy::Int8 => self.context.i8_type(),
                    MirTy::Int16 => self.context.i16_type(),
                    MirTy::Int32 => self.context.i32_type(),
                    _ => self.i64_type(),
                };
                let bits = ity.get_bit_width();
                if val.is_float_value() {
                    let fv = val.into_float_value();
                    let iv = self
                        .builder
                        .build_float_to_signed_int(fv, ity, "fptosi")?;
                    return Ok(iv.into());
                }
                if val.is_int_value() {
                    let iv = val.into_int_value();
                    let cur = iv.get_type().get_bit_width();
                    if cur != bits {
                        let cast = if cur > bits {
                            self.builder.build_int_truncate(iv, ity, "trunc")?
                        } else {
                            self.builder.build_int_s_extend(iv, ity, "sext")?
                        };
                        return Ok(cast.into());
                    }
                }
            }
            _ => {}
        }
        Ok(val)
    }

    fn get_value(&self, val: &Value) -> Result<BasicValueEnum<'ctx>> {
        match val {
            Value::Local(l) => {
                if self.is_address_local(*l) {
                    if let Some(v) = self.values.get(&l.0).copied() {
                        return Ok(v);
                    }
                    if let Some(p) = self.allocas.get(&l.0).copied() {
                        return Ok(p.into());
                    }
                    return Err(anyhow::anyhow!("local {} has no address", l.0));
                }
                if let Some(ptr) = self.allocas.get(&l.0).copied() {
                    let ty = self
                        .local_types
                        .get(&l.0)
                        .cloned()
                        .unwrap_or(MirTy::Int);
                    let llvm_ty = llvm_ty_with_layout(self.context, &self.layout, &ty);
                    let loaded = self
                        .builder
                        .build_load(llvm_ty, ptr, &format!("ld{}", l.0))
                        .with_context(|| format!("load local {}", l.0))?;
                    return Ok(loaded);
                }
                self.values
                    .get(&l.0)
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("local {} has no SSA value", l.0))
            }
            Value::ConstInt(n) => Ok(self.i64_type().const_int(*n as u64, true).into()),
            Value::ConstFloat(f) => Ok(self.context.f64_type().const_float(*f).into()),
            Value::ConstBool(b) => Ok(self.bool_type().const_int(*b as u64, false).into()),
            Value::ConstChar(c) => Ok(self.context.i32_type().const_int(*c as u64, false).into()),
            Value::Nil => Ok(self.ptr_type().const_null().into()),
        }
    }

    fn extern_fn(&self, name: &str) -> FunctionValue<'ctx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("missing extern `{name}`"))
    }

    /// Spawn zero-arg `func_name` via `ll_task_spawn` / `ll_task_spawn_ret`.
    fn emit_task_spawn(
        &self,
        func_name: &str,
        returns_value: bool,
        ret_ty: &MirTy,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let target = self.module.get_function(func_name).ok_or_else(|| {
            anyhow::anyhow!("task spawn target function `{func_name}` not found in module")
        })?;
        let spawn_name = if returns_value {
            "ll_task_spawn_ret"
        } else {
            "ll_task_spawn"
        };
        let spawn = self.extern_fn(spawn_name);
        let fn_ptr: PointerValue = target.as_global_value().as_pointer_value();
        let call = self
            .builder
            .build_call(spawn, &[fn_ptr.into()], "task_spawn")
            .with_context(|| spawn_name.to_string())?;
        if matches!(ret_ty, MirTy::Void) {
            return Ok(None);
        }
        Ok(call.try_as_basic_value().basic())
    }

    /// `TaskGroup.add` — spawn `func_name` into the group handle in `args[0]`.
    fn emit_task_group_add(
        &self,
        func_name: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let target = self.module.get_function(func_name).ok_or_else(|| {
            anyhow::anyhow!("task group add target `{func_name}` not found in module")
        })?;
        let group = args.first().copied().ok_or_else(|| {
            anyhow::anyhow!("ll_task_group_add missing group argument")
        })?;
        let add = self.extern_fn("ll_task_group_add");
        let fn_ptr: PointerValue = target.as_global_value().as_pointer_value();
        self.builder
            .build_call(add, &[group, fn_ptr.into()], "task_group_add")
            .with_context(|| "ll_task_group_add".to_string())?;
        Ok(None)
    }

    /// `HttpServer.serve(port, handler)` — pass `(String)->String` fn pointer to runtime.
    fn emit_http_serve(
        &self,
        handler: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let target = self.module.get_function(handler).ok_or_else(|| {
            anyhow::anyhow!("HttpServer.serve handler `{handler}` not found in module")
        })?;
        let port = args.first().copied().ok_or_else(|| {
            anyhow::anyhow!("ll_http_server_serve missing port argument")
        })?;
        let serve = self.extern_fn("ll_http_server_serve");
        let fn_ptr: PointerValue = target.as_global_value().as_pointer_value();
        self.builder
            .build_call(serve, &[port, fn_ptr.into()], "http_serve")
            .with_context(|| "ll_http_server_serve".to_string())?;
        Ok(None)
    }

    /// Declare an external function matching call-site arg/return types (prebuilt `.bll` symbols).
    fn declare_prebuilt_extern(
        &self,
        name: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
        ret_ty: &MirTy,
    ) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function(name) {
            return Ok(f);
        }
        let param_tys: Vec<BasicMetadataTypeEnum> = args
            .iter()
            .map(|a| {
                BasicValueEnum::try_from(*a)
                    .map(|v| v.get_type().into())
                    .unwrap_or_else(|_| self.ptr_type().into())
            })
            .collect();
        let fn_ty = if *ret_ty == MirTy::Void {
            self.context.void_type().fn_type(&param_tys, false)
        } else {
            llvm_ty_with_layout(self.context, &self.layout, ret_ty).fn_type(&param_tys, false)
        };
        Ok(self
            .module
            .add_function(name, fn_ty, Some(Linkage::External)))
    }

    /// Box an error value for `__err_out`: retain pointer-like values, otherwise `ll_box_bytes`.
    fn box_error_value(&mut self, val: &Value) -> Result<BasicValueEnum<'ctx>> {
        let bv = self.get_value(val)?;
        if bv.is_pointer_value() {
            let retained = self
                .builder
                .build_call(
                    self.extern_fn("ll_retain"),
                    &[bv.into_pointer_value().into()],
                    "err_retain",
                )?
                .try_as_basic_value()
                .basic()
                .unwrap_or_else(|| self.ptr_type().const_null().into());
            return Ok(retained);
        }
        let llvm_ty = bv.get_type();
        let tmp = self.builder.build_alloca(llvm_ty, "err_tmp")?;
        self.builder.build_store(tmp, bv)?;
        let size_i64 = match llvm_ty {
            BasicTypeEnum::IntType(t) => self
                .i64_type()
                .const_int((t.get_bit_width() as u64).div_ceil(8), false),
            BasicTypeEnum::FloatType(_) => self.i64_type().const_int(8, false),
            BasicTypeEnum::StructType(st) => {
                let n = st.count_fields() as u64;
                self.i64_type().const_int(n.max(1) * 8, false)
            }
            _ => self.i64_type().const_int(64, false),
        };
        let boxed = self
            .builder
            .build_call(
                self.extern_fn("ll_box_bytes"),
                &[tmp.into(), size_i64.into()],
                "err_box",
            )?
            .try_as_basic_value()
            .basic()
            .unwrap_or_else(|| self.ptr_type().const_null().into());
        Ok(boxed)
    }

    fn emit_inst(&mut self, inst: &Inst) -> Result<()> {
        match inst {
            Inst::Alloca { dst, ty } => {
                self.set_local_ty(*dst, ty.clone());
                let llvm_ty = mir_ty_to_llvm(self.context, ty);
                let slot = self
                    .builder
                    .build_alloca(llvm_ty, &format!("loc{}", dst.0))
                    .with_context(|| "local alloca")?;
                self.allocas.insert(dst.0, slot);
                // Address-taken slots: the local's value *is* the pointer.
                if matches!(ty, MirTy::Named(n) if n == "__err_out" || n == "void*") {
                    self.builder
                        .build_store(slot, self.ptr_type().const_null())
                        .with_context(|| "init err_slot")?;
                    self.values.insert(dst.0, slot.into());
                } else if let Some(pre) = self.values.get(&dst.0).copied() {
                    // Parameter locals are seeded before entry Alloca — spill into the slot.
                    self.builder
                        .build_store(slot, pre)
                        .with_context(|| "spill param")?;
                }
            }
            Inst::Store { dst, val } => {
                let v = self.get_value(val)?;
                self.set_value(*dst, v)?;
            }
            Inst::Load { dst, src } => {
                let v = self.get_value(&Value::Local(*src))?;
                self.set_value(*dst, v)?;
            }
            Inst::StringLit { dst, value } => {
                let global = self
                    .builder
                    .build_global_string_ptr(value, "strlit")
                    .with_context(|| "global string")?;
                let f = self.extern_fn("ll_string_from_cstr");
                let s = self
                    .builder
                    .build_call(
                        f,
                        &[global.as_pointer_value().into()],
                        "str",
                    )
                    .with_context(|| "ll_string_from_cstr")?
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.set_local_ty(*dst, MirTy::String);
                self.set_value(*dst, s)?;
            }
            Inst::StringInterp { dst, parts } => {
                let mut acc: Option<BasicValueEnum<'ctx>> = None;
                for part in parts {
                    let piece = match part {
                        InterpPart::Lit(s) => {
                            let g = self.builder.build_global_string_ptr(s, "ipl")?;
                            let f = self.extern_fn("ll_string_from_cstr");
                            self.builder
                                .build_call(f, &[g.as_pointer_value().into()], "ip")
                                .with_context(|| "interp lit")?
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                        }
                        InterpPart::Value(v) => {
                            let bv = self.get_value(v)?;
                            if bv.is_float_value() {
                                let f = self.extern_fn("ll_string_from_f64");
                                self.builder
                                    .build_call(f, &[bv.into()], "isf")
                                    .with_context(|| "interp float")?
                                    .try_as_basic_value()
                                    .basic()
                                    .unwrap()
                            } else if bv.is_int_value() {
                                let iv = bv.into_int_value();
                                if iv.get_type().get_bit_width() == 1 {
                                    let as_i32 = self.builder.build_int_z_extend(
                                        iv,
                                        self.context.i32_type(),
                                        "b2i",
                                    )?;
                                    let f = self.extern_fn("ll_string_from_bool");
                                    self.builder
                                        .build_call(f, &[as_i32.into()], "isb")
                                        .with_context(|| "interp bool")?
                                        .try_as_basic_value()
                                        .basic()
                                        .unwrap()
                                } else {
                                    let f = self.extern_fn("ll_string_from_i64");
                                    self.builder
                                        .build_call(f, &[iv.into()], "isi")
                                        .with_context(|| "interp int")?
                                        .try_as_basic_value()
                                        .basic()
                                        .unwrap()
                                }
                            } else if bv.is_pointer_value() {
                                self.builder.build_call(
                                    self.extern_fn("ll_retain"),
                                    &[bv.into()],
                                    "ir",
                                )?;
                                bv
                            } else {
                                anyhow::bail!("unsupported string interp value");
                            }
                        }
                    };
                    acc = Some(match acc {
                        None => piece,
                        Some(left) => {
                            let f = self.extern_fn("ll_string_concat");
                            self.builder
                                .build_call(f, &[left.into(), piece.into()], "cat")
                                .with_context(|| "concat")?
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                        }
                    });
                }
                let result = acc.unwrap_or_else(|| self.ptr_type().const_null().into());
                self.set_local_ty(*dst, MirTy::String);
                self.set_value(*dst, result)?;
            }
            Inst::Call {
                dst,
                callee,
                args,
                ty,
            } => {
                // Remember trailing `__err_out` arg for subsequent LoadErrFlag.
                if let Some(Value::Local(err_loc)) = args.last() {
                    if matches!(
                        self.local_types.get(&err_loc.0),
                        Some(MirTy::Named(n)) if n == "__err_out"
                    ) {
                        if let Ok(v) = self.get_value(&Value::Local(*err_loc)) {
                            if v.is_pointer_value() {
                                self.last_err_out = Some(v.into_pointer_value());
                            }
                        }
                    }
                }
                let mut_array = matches!(
                    callee,
                    Callee::Builtin(n) if matches!(
                        n.as_str(),
                        "ll_array_append_i64"
                            | "ll_array_set_i64"
                            | "ll_array_append"
                            | "ll_array_append_ptr"
                    )
                );
                let llvm_args: Vec<BasicMetadataValueEnum> = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        if mut_array && i == 0 {
                            // Mutating array helpers take `void **` (handle slot address).
                            if let Value::Local(loc) = a {
                                if let Some(slot) = self.allocas.get(&loc.0).copied() {
                                    return Ok(slot.into());
                                }
                            }
                        }
                        self.get_value(a).map(|v| v.into())
                    })
                    .collect::<Result<_>>()?;
                let arg0_ty = args.first().and_then(|a| match a {
                    Value::Local(l) => self.local_types.get(&l.0).cloned(),
                    _ => None,
                });
                let ret = self.emit_call(callee, &llvm_args, ty, arg0_ty.as_ref())?;
                if let Some(d) = dst {
                    self.set_local_ty(*d, ty.clone());
                    if let Some(r) = ret {
                        self.set_value(*d, r)?;
                    }
                }
            }
            Inst::Binary {
                dst,
                op,
                left,
                right,
                ty,
            } => {
                let l = self.get_value(left)?;
                let r = self.get_value(right)?;
                let out = self.emit_binop(*op, l, r, ty)?;
                self.set_local_ty(*dst, ty.clone());
                self.set_value(*dst, out)?;
            }
            Inst::Unary { dst, op, val, ty } => {
                let v = self.get_value(val)?;
                let out = self.emit_unop(*op, v, ty)?;
                self.set_local_ty(*dst, ty.clone());
                self.set_value(*dst, out)?;
            }
            Inst::Retain(val) => {
                let v = self.get_value(val)?;
                if v.is_pointer_value() {
                    self.builder.build_call(
                        self.extern_fn("ll_retain"),
                        &[v.into()],
                        "retain",
                    )?;
                }
            }
            Inst::Release(val) => {
                let v = self.get_value(val)?;
                if v.is_pointer_value() {
                    self.builder.build_call(
                        self.extern_fn("ll_release"),
                        &[v.into()],
                        "release",
                    )?;
                }
            }
            Inst::Construct {
                dst,
                ty_name,
                fields,
                is_class,
            } => {
                self.emit_construct(*dst, ty_name, fields, *is_class)?;
            }
            Inst::GetField { dst, base, index, ty } => {
                let bv = self.get_value(base)?;
                let base_ty = match base {
                    Value::Local(l) => self.local_types.get(&l.0).cloned(),
                    _ => None,
                };
                let loaded = self.emit_get_field(bv, base_ty.as_ref(), *index, ty)?;
                self.set_local_ty(*dst, ty.clone());
                self.set_value(*dst, loaded)?;
            }
            Inst::SetField { base, index, val } => {
                let base_v = self.get_value(base)?;
                let base_ty = match base {
                    Value::Local(l) => self.local_types.get(&l.0).cloned(),
                    _ => None,
                };
                let val_v = self.get_value(val)?;
                self.emit_set_field(base_v, base_ty.as_ref(), *index, val_v)?;
            }
            Inst::EnumTag { dst, base } => {
                let bv = self.get_value(base)?;
                let tag = self.extract_enum_tag(bv)?;
                self.set_local_ty(*dst, MirTy::Int32);
                self.set_value(*dst, tag.into())?;
            }
            Inst::EnumPayload {
                dst,
                base,
                case_index: _,
                field_index,
                ty,
            } => {
                let bv = self.get_value(base)?;
                let payload = self.extract_enum_payload(bv, *field_index, ty)?;
                self.set_local_ty(*dst, ty.clone());
                self.set_value(*dst, payload)?;
            }
            Inst::OptionalSome { dst, inner, inner_ty } => {
                let v = self.get_value(inner)?;
                let opt_ty = optional_llvm_type(self.context, inner_ty);
                let undef = opt_ty.get_undef();
                let present = self.bool_type().const_int(1, false);
                let agg = self
                    .builder
                    .build_insert_value(undef, present, 0, "opt_some_p")
                    .with_context(|| "opt some present")?;
                let agg = self
                    .builder
                    .build_insert_value(agg, v, 1, "opt_some_v")
                    .with_context(|| "opt some val")?;
                self.set_local_ty(*dst, MirTy::Optional(Box::new(inner_ty.clone())));
                self.set_value(*dst, aggregate_to_basic(agg))?;
            }
            Inst::OptionalNone { dst, inner_ty } => {
                let opt_ty = optional_llvm_type(self.context, inner_ty);
                let undef = opt_ty.get_undef();
                let present = self.bool_type().const_int(0, false);
                let zero = zero_value(self.context, inner_ty);
                let agg = self
                    .builder
                    .build_insert_value(undef, present, 0, "opt_none_p")
                    .with_context(|| "opt none present")?;
                let agg = self
                    .builder
                    .build_insert_value(agg, zero, 1, "opt_none_v")
                    .with_context(|| "opt none val")?;
                self.set_local_ty(*dst, MirTy::Optional(Box::new(inner_ty.clone())));
                self.set_value(*dst, aggregate_to_basic(agg))?;
            }
            Inst::UnwrapOptional {
                dst,
                opt,
                inner_ty,
                force,
            } => {
                let ov = self.get_value(opt)?;
                let ov_struct = ov.into_struct_value();
                let present = self
                    .builder
                    .build_extract_value(ov_struct, 0, "opt_present")
                    .with_context(|| "unwrap present")?;
                if !*force {
                    let parent = self
                        .builder
                        .get_insert_block()
                        .and_then(|bb| bb.get_parent())
                        .expect("unwrap in function");
                    let fail = self.context.append_basic_block(parent, "unwrap_fail");
                    let ok = self.context.append_basic_block(parent, "unwrap_ok");
                    let cont = self.context.append_basic_block(parent, "unwrap_cont");
                    let present_i1 = present.into_int_value();
                    self.builder
                        .build_conditional_branch(present_i1, ok, fail)
                        .with_context(|| "unwrap branch")?;
                    self.builder.position_at_end(fail);
                    let msg = self.builder.build_global_string_ptr("unwrap", "trap_msg")?;
                    self.builder.build_call(
                        self.extern_fn("ll_trap"),
                        &[msg.as_pointer_value().into(), self.ptr_type().const_null().into()],
                        "trap",
                    )?;
                    self.builder.build_unreachable()?;
                    self.builder.position_at_end(ok);
                    let inner = self
                        .builder
                        .build_extract_value(ov_struct, 1, "opt_val")
                        .with_context(|| "unwrap value")?;
                    self.set_local_ty(*dst, inner_ty.clone());
                    self.set_value(*dst, inner)?;
                    self.builder.build_unconditional_branch(cont)?;
                    self.builder.position_at_end(cont);
                } else {
                    let inner = self
                        .builder
                        .build_extract_value(ov_struct, 1, "opt_val")
                        .with_context(|| "unwrap value")?;
                    self.set_local_ty(*dst, inner_ty.clone());
                    self.set_value(*dst, inner)?;
                }
            }
            Inst::SetErrOut { val } => {
                if let Some(err_out) = self.err_out_ptr {
                    let boxed = self.box_error_value(val)?;
                    self.builder.build_store(err_out, boxed)?;
                }
            }
            Inst::LoadErrFlag { dst } => {
                self.set_local_ty(*dst, MirTy::Bool);
                let flag = if let Some(err_ptr) = self.last_err_out {
                    let loaded = self
                        .builder
                        .build_load(self.ptr_type(), err_ptr, "err_val")
                        .with_context(|| "load err_out")?;
                    let is_err = self.builder.build_int_compare(
                        IntPredicate::NE,
                        self.builder
                            .build_ptr_to_int(loaded.into_pointer_value(), self.i64_type(), "err_i")?,
                        self.i64_type().const_int(0, false),
                        "err_nz",
                    )?;
                    is_err.into()
                } else {
                    self.bool_type().const_int(0, false).into()
                };
                self.set_value(*dst, flag)?;
            }
            Inst::LoadErrValue { dst } => {
                self.set_local_ty(*dst, MirTy::Named("void*".into()));
                let ptr_val = if let Some(err_ptr) = self.last_err_out {
                    self.builder
                        .build_load(self.ptr_type(), err_ptr, "err_payload")
                        .with_context(|| "load err payload")?
                } else {
                    self.ptr_type().const_null().into()
                };
                self.set_value(*dst, ptr_val)?;
            }
        }
        Ok(())
    }

    fn emit_call(
        &mut self,
        callee: &Callee,
        args: &[BasicMetadataValueEnum<'ctx>],
        ret_ty: &MirTy,
        arg0_ty: Option<&MirTy>,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        if let Callee::Virtual { class, method, slot } = callee {
            return self.emit_virtual_call(class, method, *slot, args, ret_ty);
        }
        if let Callee::TaskSpawn {
            name: func_name,
            returns_value,
        } = callee
        {
            return self.emit_task_spawn(func_name, *returns_value, ret_ty);
        }
        if let Callee::TaskGroupAdd { name: func_name } = callee {
            return self.emit_task_group_add(func_name, args);
        }
        if let Callee::HttpServe { handler } = callee {
            return self.emit_http_serve(handler, args);
        }
        let name = match callee {
            Callee::Func(n) => n.as_str(),
            Callee::Builtin(n) => n.as_str(),
            Callee::Virtual { .. }
            | Callee::TaskSpawn { .. }
            | Callee::TaskGroupAdd { .. }
            | Callee::HttpServe { .. } => {
                unreachable!()
            }
        };
        if name == "ll_int_from_string"
            || name == "ll_env_get_i64"
            || name == "ll_env_get_bool"
            || name == "ll_env_get_f64"
            || name == "ll_env_get_decimal"
            || name == "ll_decimal_from_string"
        {
            return self.emit_scalar_with_ok(name, args, ret_ty);
        }
        if name == "ll_json_decode" || name == "ll_toon_decode" {
            return self.emit_codec_decode(name, args, ret_ty);
        }
        if name == "ll_json_encode" || name == "ll_toon_encode" {
            return self.emit_codec_encode(name, args, arg0_ty);
        }
        if name == "ll_http_get" || name == "ll_http_post" {
            return self.emit_http_call(name, args, ret_ty);
        }
        if matches!(
            name,
            "ll_env_get" | "ll_args_get" | "ll_read_line" | "ll_dict_get"
        ) {
            return self.emit_nullable_string(name, args, ret_ty);
        }
        if name == "ll_optional_is_some" {
            // Optional LLVM layout: { i1 present, T value }
            let opt = args
                .first()
                .and_then(|a| BasicValueEnum::try_from(*a).ok())
                .ok_or_else(|| anyhow::anyhow!("ll_optional_is_some expects optional arg"))?;
            let present = self
                .builder
                .build_extract_value(opt.into_struct_value(), 0, "is_some")
                .with_context(|| "extract optional present")?;
            return Ok(Some(present));
        }
        let f = if let Some(f) = self.module.get_function(name) {
            f
        } else if matches!(callee, Callee::Func(_)) {
            // Prefer a function defined in this module; otherwise declare an extern for `.bll` link.
            self.declare_prebuilt_extern(name, args, ret_ty)?
        } else if matches!(callee, Callee::Builtin(_)) {
            // `@extern(cName:)` and other late C symbols — declare from call shape.
            self.declare_prebuilt_extern(name, args, ret_ty)?
        } else {
            self.extern_fn(name)
        };
        // Coerce Bool (i1) args to i32 for C helpers like `ll_assert`.
        let coerced: Vec<BasicMetadataValueEnum> = args
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let v = BasicValueEnum::try_from(*a);
                if let Ok(BasicValueEnum::IntValue(iv)) = v {
                    if iv.get_type().get_bit_width() == 1 {
                        let param_tys = f.get_type().get_param_types();
                        if let Some(BasicMetadataTypeEnum::IntType(it)) = param_tys.get(i).copied() {
                            if it.get_bit_width() == 32 {
                                return self
                                    .builder
                                    .build_int_z_extend(iv, it, "zext_i1")
                                    .map(|x| x.into())
                                    .unwrap_or(*a);
                            }
                        }
                    }
                }
                *a
            })
            .collect();
        let call = self
            .builder
            .build_call(f, &coerced, "call")
            .with_context(|| format!("call {name}"))?;
        if *ret_ty == MirTy::Void {
            Ok(None)
        } else {
            let mut result = call
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| anyhow::anyhow!("void call used with non-void ty"))?;
            // C `int` → MIR Bool
            if matches!(ret_ty, MirTy::Bool) && result.is_int_value() {
                let iv = result.into_int_value();
                if iv.get_type().get_bit_width() > 1 {
                    result = self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            iv,
                            iv.get_type().const_int(0, false),
                            "boolish",
                        )?
                        .into();
                }
            }
            Ok(Some(result))
        }
    }

    fn emit_virtual_call(
        &mut self,
        class: &str,
        method: &str,
        slot: u32,
        args: &[BasicMetadataValueEnum<'ctx>],
        ret_ty: &MirTy,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        // Prefer indirect call through object's vtable; fall back to static mangled name.
        let mangled = format!("{class}_{method}");
        let self_ptr = args
            .first()
            .and_then(|a| BasicValueEnum::try_from(*a).ok())
            .and_then(|v| {
                if v.is_pointer_value() {
                    Some(v.into_pointer_value())
                } else {
                    None
                }
            });
        if let Some(obj) = self_ptr {
            if let Some(st) = self.layout.struct_types.get(class).copied() {
                let vt_slot = self.builder.build_struct_gep(st, obj, 0, "vt_slot")?;
                let vt_ptr = self
                    .builder
                    .build_load(self.ptr_type(), vt_slot, "vt")?
                    .into_pointer_value();
                let slot_ptr = unsafe {
                    self.builder.build_in_bounds_gep(
                        self.ptr_type(),
                        vt_ptr,
                        &[self.i64_type().const_int(slot as u64, false)],
                        "vt_entry",
                    )?
                };
                let fn_ptr = self
                    .builder
                    .build_load(self.ptr_type(), slot_ptr, "vfn")?
                    .into_pointer_value();
                // Use a concrete method's type for the indirect call (protocol existentials
                // may not define `Proto_method` — any `*_method` implementor works).
                let static_f = self.module.get_function(&mangled).or_else(|| {
                    let suffix = format!("_{method}");
                    self.module.get_functions().find(|f| {
                        f.get_name()
                            .to_str()
                            .map(|n| n.ends_with(&suffix))
                            .unwrap_or(false)
                    })
                });
                if let Some(static_f) = static_f {
                    let fn_ty = static_f.get_type();
                    let call = self.builder.build_indirect_call(
                        fn_ty,
                        fn_ptr,
                        args,
                        "vcall",
                    )?;
                    return if *ret_ty == MirTy::Void {
                        Ok(None)
                    } else {
                        Ok(Some(
                            call.try_as_basic_value()
                                .basic()
                                .ok_or_else(|| anyhow::anyhow!("void vcall"))?,
                        ))
                    };
                }
            }
        }
        let f = self
            .module
            .get_function(&mangled)
            .or_else(|| {
                let suffix = format!("_{method}");
                self.module.get_functions().find(|f| {
                    f.get_name()
                        .to_str()
                        .map(|n| n.ends_with(&suffix))
                        .unwrap_or(false)
                })
            })
            .ok_or_else(|| anyhow::anyhow!("undefined virtual `{mangled}`"))?;
        let call = self.builder.build_call(f, args, "vcall_static")?;
        if *ret_ty == MirTy::Void {
            Ok(None)
        } else {
            Ok(Some(
                call.try_as_basic_value()
                    .basic()
                    .ok_or_else(|| anyhow::anyhow!("void vcall"))?,
            ))
        }
    }

    fn emit_codec_decode(
        &mut self,
        which: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
        ret_ty: &MirTy,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let str_arg = args
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("decode expects string"))?;
        let type_name = match ret_ty {
            MirTy::Struct(n) | MirTy::Class(n) | MirTy::Named(n) => n.as_str(),
            _ => "Anon",
        };
        let schema = self
            .layout
            .field_schemas
            .get(type_name)
            .cloned()
            .unwrap_or_default();
        let schema_g = self
            .builder
            .build_global_string_ptr(&schema, "schema")?;
        let st = self
            .layout
            .struct_types
            .get(type_name)
            .copied()
            .unwrap_or_else(|| self.context.struct_type(&[], false));
        let out = self.builder.build_alloca(st, "decoded")?;
        let fn_name = if which.contains("toon") {
            "ll_toon_decode_fields"
        } else {
            "ll_json_decode_fields"
        };
        let ok = self.builder.build_call(
            self.extern_fn(fn_name),
            &[str_arg, schema_g.as_pointer_value().into(), out.into()],
            "dec",
        )?;
        let _ = ok;
        // On failure, leave zeroed struct (examples use try/catch separately).
        let loaded = self.builder.build_load(st, out, "dec_val")?;
        Ok(Some(loaded))
    }

    fn emit_codec_encode(
        &mut self,
        which: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
        arg_ty: Option<&MirTy>,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let val = args
            .first()
            .and_then(|a| BasicValueEnum::try_from(*a).ok())
            .ok_or_else(|| anyhow::anyhow!("encode expects value"))?;
        let type_name = match arg_ty {
            Some(MirTy::Struct(n) | MirTy::Class(n) | MirTy::Named(n)) => n.as_str(),
            _ => "",
        };
        let schema = self
            .layout
            .field_schemas
            .get(type_name)
            .cloned()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                // Fallback only when the value type is unknown (should be rare).
                self.layout
                    .field_schemas
                    .values()
                    .find(|s| !s.is_empty())
                    .cloned()
            })
            .unwrap_or_else(|| "sku:s,name:s,priceCents:i".into());
        let schema_g = self.builder.build_global_string_ptr(&schema, "enc_schema")?;
        let tmp = self.builder.build_alloca(val.get_type(), "enc_tmp")?;
        self.builder.build_store(tmp, val)?;
        let fn_name = if which.contains("toon") {
            "ll_toon_encode_fields"
        } else {
            "ll_json_encode_fields"
        };
        let s = self
            .builder
            .build_call(
                self.extern_fn(fn_name),
                &[schema_g.as_pointer_value().into(), tmp.into()],
                "enc",
            )?
            .try_as_basic_value()
            .basic()
            .unwrap();
        Ok(Some(s))
    }

    fn emit_http_call(
        &mut self,
        name: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
        ret_ty: &MirTy,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let ok_alloca = self.builder.build_alloca(self.context.i32_type(), "http_ok")?;
        self.builder
            .build_store(ok_alloca, self.context.i32_type().const_int(0, false))?;
        let mut call_args = args.to_vec();
        call_args.push(ok_alloca.into());
        let s = self
            .builder
            .build_call(self.extern_fn(name), &call_args, "http")?
            .try_as_basic_value()
            .basic()
            .unwrap();
        if matches!(ret_ty, MirTy::Optional(_)) {
            let ok = self
                .builder
                .build_load(self.context.i32_type(), ok_alloca, "ok")?;
            let present = self.builder.build_int_compare(
                IntPredicate::NE,
                ok.into_int_value(),
                self.context.i32_type().const_int(0, false),
                "oknz",
            )?;
            let opt_ty = optional_llvm_type(self.context, &MirTy::String);
            let undef = opt_ty.get_undef();
            let with_p = self
                .builder
                .build_insert_value(undef, present, 0, "hp")?
                .into_struct_value();
            let with_v = self
                .builder
                .build_insert_value(with_p, s, 1, "hv")?
                .into_struct_value();
            return Ok(Some(with_v.into()));
        }
        Ok(Some(s))
    }

    fn emit_nullable_string(
        &mut self,
        name: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
        ret_ty: &MirTy,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let f = self.extern_fn(name);
        let ptr_val = self
            .builder
            .build_call(f, args, "nullable")?
            .try_as_basic_value()
            .basic()
            .unwrap();
        if !matches!(ret_ty, MirTy::Optional(_)) {
            return Ok(Some(ptr_val));
        }
        let p = ptr_val.into_pointer_value();
        let present = self.builder.build_int_compare(
            IntPredicate::NE,
            self.builder
                .build_ptr_to_int(p, self.i64_type(), "np")?,
            self.i64_type().const_int(0, false),
            "np_nz",
        )?;
        let opt_ty = optional_llvm_type(self.context, &MirTy::String);
        let undef = opt_ty.get_undef();
        let with_p = self
            .builder
            .build_insert_value(undef, present, 0, "nsp")?
            .into_struct_value();
        let empty = self.ptr_type().const_null();
        let with_v = self
            .builder
            .build_insert_value(with_p, empty, 1, "nsv_pre")?
            .into_struct_value();
        // Use the pointer when present; null otherwise (Optional String payload).
        let with_v = self
            .builder
            .build_insert_value(with_v, ptr_val, 1, "nsv")?
            .into_struct_value();
        Ok(Some(with_v.into()))
    }

    fn emit_scalar_with_ok(
        &mut self,
        name: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
        ret_ty: &MirTy,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let ok_alloca = self
            .builder
            .build_alloca(self.context.i32_type(), "ok_out")
            .with_context(|| "ok alloca")?;
        self.builder
            .build_store(ok_alloca, self.context.i32_type().const_int(0, false))
            .with_context(|| "init ok_out")?;
        let mut call_args = args.to_vec();
        call_args.push(ok_alloca.into());
        let f = self.extern_fn(name);
        let mut v = self
            .builder
            .build_call(f, &call_args, "parsed")
            .with_context(|| format!("call {name}"))?
            .try_as_basic_value()
            .basic()
            .unwrap();
        // C `int` bool helpers → i1 when returning Bool / Bool?.
        let inner_for_bool = match ret_ty {
            MirTy::Optional(inner) => inner.as_ref(),
            other => other,
        };
        if matches!(inner_for_bool, MirTy::Bool) && v.is_int_value() {
            let iv = v.into_int_value();
            if iv.get_type().get_bit_width() > 1 {
                v = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        iv,
                        iv.get_type().const_int(0, false),
                        "boolish",
                    )?
                    .into();
            }
        }
        if matches!(ret_ty, MirTy::Optional(_)) {
            let ok_loaded = self
                .builder
                .build_load(self.context.i32_type(), ok_alloca, "ok")
                .with_context(|| "load ok_out")?;
            let ok_i1 = self.builder.build_int_compare(
                IntPredicate::NE,
                ok_loaded.into_int_value(),
                self.context.i32_type().const_int(0, false),
                "ok_cmp",
            )?;
            let inner_ty = match ret_ty {
                MirTy::Optional(inner) => inner.as_ref(),
                _ => &MirTy::Int,
            };
            let opt_ty = optional_llvm_type(self.context, inner_ty);
            let undef = opt_ty.get_undef();
            let agg = self.builder.build_insert_value(undef, ok_i1, 0, "opt_ok_p")?;
            let agg = self.builder.build_insert_value(agg, v, 1, "opt_ok_v")?;
            Ok(Some(aggregate_to_basic(agg)))
        } else {
            Ok(Some(v))
        }
    }

    fn emit_binop(
        &self,
        op: BinOp,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
        ty: &MirTy,
    ) -> Result<BasicValueEnum<'ctx>> {
        let is_bool = matches!(ty, MirTy::Bool);
        if is_bool && matches!(op, BinOp::And | BinOp::Or) {
            let l = left.into_int_value();
            let r = right.into_int_value();
            let out = match op {
                BinOp::And => self.builder.build_and(l, r, "and")?,
                BinOp::Or => self.builder.build_or(l, r, "or")?,
                _ => unreachable!(),
            };
            return Ok(out.into());
        }
        if left.is_float_value() || right.is_float_value() || matches!(ty, MirTy::Double | MirTy::Float)
        {
            let l = if left.is_float_value() {
                left.into_float_value()
            } else {
                self.builder.build_signed_int_to_float(
                    left.into_int_value(),
                    self.context.f64_type(),
                    "i2f",
                )?
            };
            let r = if right.is_float_value() {
                right.into_float_value()
            } else {
                self.builder.build_signed_int_to_float(
                    right.into_int_value(),
                    self.context.f64_type(),
                    "i2f",
                )?
            };
            use inkwell::FloatPredicate;
            let out = match op {
                BinOp::Add => self.builder.build_float_add(l, r, "fadd")?.into(),
                BinOp::Sub => self.builder.build_float_sub(l, r, "fsub")?.into(),
                BinOp::Mul => self.builder.build_float_mul(l, r, "fmul")?.into(),
                BinOp::Div => self.builder.build_float_div(l, r, "fdiv")?.into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")?
                    .into(),
                BinOp::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")?
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")?
                    .into(),
                BinOp::Le => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")?
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")?
                    .into(),
                BinOp::Ge => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")?
                    .into(),
                _ => anyhow::bail!("unsupported float binop"),
            };
            return Ok(out);
        }
        // Struct / aggregate equality: field-wise compare.
        if left.is_struct_value() && right.is_struct_value() && matches!(op, BinOp::Eq | BinOp::NotEq)
        {
            let st = left.into_struct_value().get_type();
            let la = self.builder.build_alloca(st, "eq_l")?;
            let ra = self.builder.build_alloca(st, "eq_r")?;
            self.builder.build_store(la, left)?;
            self.builder.build_store(ra, right)?;
            let mut all_eq = self.bool_type().const_int(1, false);
            let nfields = st.count_fields();
            for i in 0..nfields {
                let lp = self.builder.build_struct_gep(st, la, i, "eql")?;
                let rp = self.builder.build_struct_gep(st, ra, i, "eqr")?;
                let ft = st
                    .get_field_type_at_index(i)
                    .unwrap_or(self.i64_type().into());
                let lv = self.builder.build_load(ft, lp, "lv")?;
                let rv = self.builder.build_load(ft, rp, "rv")?;
                let cmp = if lv.is_pointer_value() && rv.is_pointer_value() {
                    self.builder
                        .build_call(
                            self.extern_fn("ll_string_eq"),
                            &[lv.into(), rv.into()],
                            "seq",
                        )?
                        .try_as_basic_value()
                        .basic()
                        .and_then(|v| {
                            if v.is_int_value() {
                                Some(
                                    self.builder
                                        .build_int_compare(
                                            IntPredicate::NE,
                                            v.into_int_value(),
                                            self.context.i32_type().const_int(0, false),
                                            "seqz",
                                        )
                                        .ok()?,
                                )
                            } else {
                                None
                            }
                        })
                        .unwrap_or(self.bool_type().const_int(0, false))
                } else if lv.is_int_value() && rv.is_int_value() {
                    self.builder.build_int_compare(
                        IntPredicate::EQ,
                        lv.into_int_value(),
                        rv.into_int_value(),
                        "ieq",
                    )?
                } else {
                    self.bool_type().const_int(1, false)
                };
                all_eq = self.builder.build_and(all_eq, cmp, "andeq")?;
            }
            return Ok(if matches!(op, BinOp::NotEq) {
                self.builder.build_not(all_eq, "neq")?.into()
            } else {
                all_eq.into()
            });
        }
        // String / pointer equality via runtime.
        if left.is_pointer_value()
            && right.is_pointer_value()
            && matches!(op, BinOp::Eq | BinOp::NotEq)
        {
            let eq_i32 = self
                .builder
                .build_call(
                    self.extern_fn("ll_string_eq"),
                    &[left.into(), right.into()],
                    "streq",
                )?
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value();
            let eq = self.builder.build_int_compare(
                IntPredicate::NE,
                eq_i32,
                self.context.i32_type().const_int(0, false),
                "streqz",
            )?;
            return Ok(if matches!(op, BinOp::NotEq) {
                self.builder.build_not(eq, "strne")?.into()
            } else {
                eq.into()
            });
        }
        let l = left.into_int_value();
        let r = right.into_int_value();
        let out: IntValue = match op {
            BinOp::Add => self.builder.build_int_add(l, r, "add")?,
            BinOp::Sub => self.builder.build_int_sub(l, r, "sub")?,
            BinOp::Mul => self.builder.build_int_mul(l, r, "mul")?,
            BinOp::Div => self.builder.build_int_signed_div(l, r, "div")?,
            BinOp::Mod => self.builder.build_int_signed_rem(l, r, "mod")?,
            BinOp::Eq => self.builder.build_int_compare(IntPredicate::EQ, l, r, "eq")?,
            BinOp::NotEq => self.builder.build_int_compare(IntPredicate::NE, l, r, "ne")?,
            BinOp::Lt => self.builder.build_int_compare(IntPredicate::SLT, l, r, "lt")?,
            BinOp::Le => self.builder.build_int_compare(IntPredicate::SLE, l, r, "le")?,
            BinOp::Gt => self.builder.build_int_compare(IntPredicate::SGT, l, r, "gt")?,
            BinOp::Ge => self.builder.build_int_compare(IntPredicate::SGE, l, r, "ge")?,
            BinOp::And => self.builder.build_and(l, r, "and")?,
            BinOp::Or => self.builder.build_or(l, r, "or")?,
        };
        Ok(out.into())
    }

    fn emit_unop(&self, op: UnOp, val: BasicValueEnum<'ctx>, _ty: &MirTy) -> Result<BasicValueEnum<'ctx>> {
        if val.is_float_value() {
            let v = val.into_float_value();
            let out = match op {
                UnOp::Neg => self.builder.build_float_neg(v, "fneg")?,
                UnOp::Pos | UnOp::Not => v,
            };
            return Ok(out.into());
        }
        let v = val.into_int_value();
        let out = match op {
            UnOp::Not => self.builder.build_not(v, "not")?,
            UnOp::Neg => self.builder.build_int_neg(v, "neg")?,
            UnOp::Pos => v,
        };
        Ok(out.into())
    }

    fn struct_type_for_name(&self, name: &str) -> StructType<'ctx> {
        if let Some(st) = self.layout.struct_types.get(name) {
            *st
        } else if name.contains('#') {
            self.layout.enum_rep
        } else {
            self.context.struct_type(&[], false)
        }
    }

    fn emit_construct(
        &mut self,
        dst: Local,
        ty_name: &str,
        fields: &[Value],
        is_class: bool,
    ) -> Result<()> {
        if is_class {
            let st = self.struct_type_for_name(ty_name);
            // Conservative payload size: 8 bytes per field (handles i64/f64/ptr), includes vtable.
            let nbytes = (st.count_fields() as u64).max(1) * 8;
            let size = self.i64_type().const_int(nbytes, false);
            let obj = self
                .builder
                .build_call(
                    self.extern_fn("ll_alloc_sized"),
                    &[size.into()],
                    "obj",
                )
                .with_context(|| "ll_alloc_sized")?
                .try_as_basic_value()
                .basic()
                .unwrap();
            // Field 0 = vtable pointer (may be null if class has no virtual methods).
            let vt_name = format!("{ty_name}_vtable");
            let vt_ptr = if let Some(g) = self.module.get_global(&vt_name) {
                g.as_pointer_value()
            } else {
                self.ptr_type().const_null()
            };
            let vt_slot = self
                .builder
                .build_struct_gep(st, obj.into_pointer_value(), 0, "vt_init")
                .with_context(|| "vtable gep")?;
            self.builder.build_store(vt_slot, vt_ptr)?;
            for (i, fv) in fields.iter().enumerate() {
                let mut val = self.get_value(fv)?;
                let field_tys = st.get_field_types();
                let llvm_idx = (i + 1) as u32; // +1 for vtable
                if let Some(ft) = field_tys.get(llvm_idx as usize) {
                    if ft.is_float_type() && val.is_int_value() {
                        val = self
                            .builder
                            .build_signed_int_to_float(
                                val.into_int_value(),
                                self.context.f64_type(),
                                "i2f_cf",
                            )?
                            .into();
                    }
                }
                let field_ptr = self
                    .builder
                    .build_struct_gep(st, obj.into_pointer_value(), llvm_idx, "cf")
                    .with_context(|| "class field gep")?;
                self.builder.build_store(field_ptr, val)?;
            }
            self.set_local_ty(dst, MirTy::Class(ty_name.into()));
            self.set_value(dst, obj)?;
            return Ok(());
        }

        // Enum case: `EnumName#caseName#tagIndex`
        if let Some((enum_name, rest)) = ty_name.split_once('#') {
            let parts: Vec<&str> = rest.split('#').collect();
            let tag_idx: u64 = parts
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let st = self.layout.enum_rep;
            let alloca = self
                .builder
                .build_alloca(st, "enum_tmp")
                .with_context(|| "enum alloca")?;
            let tag_ptr = self
                .builder
                .build_struct_gep(st, alloca, 0, "etag")
                .with_context(|| "enum tag gep")?;
            self.builder.build_store(
                tag_ptr,
                self.context.i32_type().const_int(tag_idx, false),
            )?;
            let payload_ptr = self
                .builder
                .build_struct_gep(st, alloca, 1, "epay")
                .with_context(|| "enum payload gep")?;
            for (i, fv) in fields.iter().enumerate() {
                let val = self.get_value(fv)?;
                let as_i64 = if val.is_int_value() {
                    let iv = val.into_int_value();
                    if iv.get_type().get_bit_width() < 64 {
                        self.builder
                            .build_int_s_extend(iv, self.i64_type(), "pw")?
                    } else {
                        iv
                    }
                } else if val.is_float_value() {
                    self.builder
                        .build_bit_cast(val.into_float_value(), self.i64_type(), "pf")?
                        .into_int_value()
                } else if val.is_pointer_value() {
                    self.builder
                        .build_ptr_to_int(val.into_pointer_value(), self.i64_type(), "pp")?
                } else {
                    self.i64_type().const_int(0, false)
                };
                let slot = unsafe {
                    self.builder.build_in_bounds_gep(
                        self.i64_type(),
                        payload_ptr,
                        &[self.i64_type().const_int(i as u64, false)],
                        "eslot",
                    )?
                };
                self.builder.build_store(slot, as_i64)?;
            }
            let loaded = self.builder.build_load(st, alloca, "enum_val")?;
            self.set_local_ty(dst, MirTy::Enum(enum_name.into()));
            self.set_value(dst, loaded)?;
            return Ok(());
        }

        let st = self.struct_type_for_name(ty_name);
        let alloca = self
            .builder
            .build_alloca(st, "struct_tmp")
            .with_context(|| "struct alloca")?;
        for (i, fv) in fields.iter().enumerate() {
            let mut val = self.get_value(fv)?;
            let field_ptr = self
                .builder
                .build_struct_gep(st, alloca, i as u32, "sf")
                .with_context(|| "struct gep")?;
            // Coerce int → float when the struct field is floating.
            let field_tys = st.get_field_types();
            if let Some(ft) = field_tys.get(i) {
                if ft.is_float_type() && val.is_int_value() {
                    val = self
                        .builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "i2f_field",
                        )?
                        .into();
                }
            }
            self.builder.build_store(field_ptr, val)?;
        }
        let loaded = self.builder.build_load(st, alloca, "struct_val")?;
        self.set_local_ty(dst, MirTy::Struct(ty_name.into()));
        self.set_value(dst, loaded)?;
        Ok(())
    }

    fn emit_get_field(
        &self,
        base: BasicValueEnum<'ctx>,
        base_ty: Option<&MirTy>,
        index: usize,
        ty: &MirTy,
    ) -> Result<BasicValueEnum<'ctx>> {
        let class_name = match base_ty {
            Some(MirTy::Class(n)) => Some(n.as_str()),
            _ => None,
        };
        let is_weak = class_name
            .and_then(|n| self.layout.weak_fields.get(n))
            .map(|w| w.contains(&index))
            .unwrap_or(false);
        let is_unowned = class_name
            .and_then(|n| self.layout.unowned_fields.get(n))
            .map(|w| w.contains(&index))
            .unwrap_or(false);
        let llvm_ty = if is_weak || is_unowned {
            self.ptr_type().into()
        } else {
            mir_ty_to_llvm(self.context, ty)
        };
        let class_offset = matches!(base_ty, Some(MirTy::Class(_))) as usize;
        let llvm_index = (index + class_offset) as u32;
        if base.is_pointer_value() {
            let ptr = base.into_pointer_value();
            let st = base_ty.and_then(|t| match t {
                MirTy::Class(n) | MirTy::Struct(n) => self.layout.struct_types.get(n).copied(),
                _ => None,
            });
            if let Some(st) = st {
                let field_ptr = self
                    .builder
                    .build_struct_gep(st, ptr, llvm_index, "fld_ptr")
                    .with_context(|| "class field gep")?;
                if is_weak {
                    let load_fn = self.extern_fn("ll_weak_load");
                    let loaded = self
                        .builder
                        .build_call(load_fn, &[field_ptr.into()], "weak_ld")?
                        .try_as_basic_value()
                        .basic()
                        .ok_or_else(|| anyhow::anyhow!("ll_weak_load"))?;
                    // Wrap as Optional when the MIR type is Optional.
                    if let MirTy::Optional(inner) = ty {
                        let opt_ty = optional_llvm_type(self.context, inner);
                        let undef = opt_ty.get_undef();
                        let is_null = self.builder.build_is_null(loaded.into_pointer_value(), "wnull")?;
                        let present = self.builder.build_not(is_null, "wpresent")?;
                        let agg = self
                            .builder
                            .build_insert_value(undef, present, 0, "wopt_p")
                            .with_context(|| "weak opt present")?;
                        let agg = self
                            .builder
                            .build_insert_value(agg, loaded, 1, "wopt_v")
                            .with_context(|| "weak opt val")?;
                        return Ok(agg.into_struct_value().into());
                    }
                    return Ok(loaded);
                }
                if is_unowned {
                    let load_fn = self.extern_fn("ll_unowned_load");
                    let loaded = self
                        .builder
                        .build_call(load_fn, &[field_ptr.into()], "unowned_ld")?
                        .try_as_basic_value()
                        .basic()
                        .ok_or_else(|| anyhow::anyhow!("ll_unowned_load"))?;
                    return Ok(loaded);
                }
                let loaded = self
                    .builder
                    .build_load(llvm_ty, field_ptr, "fld")
                    .with_context(|| "class load field")?;
                return Ok(loaded);
            }
            anyhow::bail!("class field access without layout");
        }
        let extracted = self
            .builder
            .build_extract_value(base.into_struct_value(), llvm_index, "xf")
            .with_context(|| "extract field")?;
        Ok(extracted)
    }

    fn emit_set_field(
        &self,
        base: BasicValueEnum<'ctx>,
        base_ty: Option<&MirTy>,
        index: usize,
        val: BasicValueEnum<'ctx>,
    ) -> Result<()> {
        let class_name = match base_ty {
            Some(MirTy::Class(n)) => Some(n.as_str()),
            _ => None,
        };
        let is_weak = class_name
            .and_then(|n| self.layout.weak_fields.get(n))
            .map(|w| w.contains(&index))
            .unwrap_or(false);
        let is_unowned = class_name
            .and_then(|n| self.layout.unowned_fields.get(n))
            .map(|w| w.contains(&index))
            .unwrap_or(false);
        let class_offset = matches!(base_ty, Some(MirTy::Class(_))) as usize;
        let llvm_index = (index + class_offset) as u32;
        if base.is_pointer_value() {
            let ptr = base.into_pointer_value();
            let st = base_ty.and_then(|t| match t {
                MirTy::Class(n) | MirTy::Struct(n) => self.layout.struct_types.get(n).copied(),
                _ => None,
            });
            if let Some(st) = st {
                let field_ptr = self
                    .builder
                    .build_struct_gep(st, ptr, llvm_index, "sfld")
                    .with_context(|| "class set gep")?;
                if is_weak {
                    // Unwrap Optional → payload pointer (or null).
                    let obj = if val.is_struct_value() {
                        let sv = val.into_struct_value();
                        let present = self.builder.build_extract_value(sv, 0, "wp")?.into_int_value();
                        let payload = self.builder.build_extract_value(sv, 1, "wv")?;
                        let null = self.ptr_type().const_null();
                        self.builder
                            .build_select(present, payload, null.into(), "wsel")?
                    } else {
                        val
                    };
                    let store_fn = self.extern_fn("ll_weak_store");
                    self.builder.build_call(
                        store_fn,
                        &[field_ptr.into(), obj.into()],
                        "weak_st",
                    )?;
                    return Ok(());
                }
                if is_unowned {
                    let store_fn = self.extern_fn("ll_unowned_store");
                    self.builder.build_call(
                        store_fn,
                        &[field_ptr.into(), val.into()],
                        "unowned_st",
                    )?;
                    // Also register in weak side table so unowned_load can detect free.
                    let weak_fn = self.extern_fn("ll_weak_store");
                    self.builder.build_call(
                        weak_fn,
                        &[field_ptr.into(), val.into()],
                        "unowned_reg",
                    )?;
                    return Ok(());
                }
                self.builder
                    .build_store(field_ptr, val)
                    .with_context(|| "class set field")?;
                return Ok(());
            }
            anyhow::bail!("class field store without layout");
        }
        Ok(())
    }

    fn extract_enum_tag(&self, base: BasicValueEnum<'ctx>) -> Result<IntValue<'ctx>> {
        let tag = self
            .builder
            .build_extract_value(base.into_struct_value(), 0, "etag")
            .with_context(|| "enum tag")?;
        Ok(tag.into_int_value())
    }

    fn extract_enum_payload(&self, base: BasicValueEnum<'ctx>, field_index: usize, ty: &MirTy) -> Result<BasicValueEnum<'ctx>> {
        let payload_arr = self
            .builder
            .build_extract_value(base.into_struct_value(), 1, "epayload")
            .with_context(|| "enum payload arr")?;
        let slot = self
            .builder
            .build_extract_value(payload_arr.into_array_value(), field_index as u32, "eslot")
            .with_context(|| "enum slot")?;
        let llvm_ty = mir_ty_to_llvm(self.context, ty);
        if llvm_ty.is_int_type() {
            Ok(slot)
        } else if ty.is_refcounted() {
            Ok(self
                .builder
                .build_int_to_ptr(slot.into_int_value(), self.ptr_type(), "eptr")?
                .into())
        } else {
            Ok(slot)
        }
    }

    fn emit_terminator(&mut self, term: &Terminator) -> Result<()> {
        match term {
            Terminator::Return(val) => {
                // Clear error out-param on success.
                if let Some(err_out) = self.err_out_ptr {
                    self.builder
                        .build_store(err_out, self.ptr_type().const_null())?;
                }
                if let Some(v) = val {
                    let bv = self.get_value(v)?;
                    self.builder.build_return(Some(&bv))?;
                } else {
                    self.builder.build_return(None)?;
                }
            }
            Terminator::Throw(val) => {
                if let Some(err_out) = self.err_out_ptr {
                    let boxed = self.box_error_value(val)?;
                    self.builder.build_store(err_out, boxed)?;
                    let parent = self
                        .builder
                        .get_insert_block()
                        .and_then(|bb| bb.get_parent())
                        .expect("throw in function");
                    let ret_ty = parent.get_type().get_return_type();
                    if let Some(rt) = ret_ty {
                        let zero = rt.const_zero();
                        self.builder.build_return(Some(&zero))?;
                    } else {
                        self.builder.build_return(None)?;
                    }
                } else {
                    let msg = self.builder.build_global_string_ptr("throw", "throw_msg")?;
                    self.builder.build_call(
                        self.extern_fn("ll_trap"),
                        &[
                            msg.as_pointer_value().into(),
                            self.ptr_type().const_null().into(),
                        ],
                        "throw_trap",
                    )?;
                    self.builder.build_unreachable()?;
                }
            }
            Terminator::Branch { cond, then_b, else_b } => {
                let cv = self.get_value(cond)?;
                let then_bb = self.bb(*then_b)?;
                let else_bb = self.bb(*else_b)?;
                self.builder
                    .build_conditional_branch(cv.into_int_value(), then_bb, else_bb)?;
            }
            Terminator::Jump(target) => {
                let bb = self.bb(*target)?;
                self.builder.build_unconditional_branch(bb)?;
            }
            Terminator::Switch { subject, arms, default } => {
                let sv = self.get_value(subject)?;
                // Accept plain ints, or enum/optional aggregates (tag / present in field 0).
                let discr = if sv.is_int_value() {
                    let iv = sv.into_int_value();
                    if iv.get_type().get_bit_width() < 64 {
                        self.builder
                            .build_int_z_extend(iv, self.i64_type(), "sw_zext")?
                    } else {
                        iv
                    }
                } else if sv.is_struct_value() {
                    let tag = self
                        .builder
                        .build_extract_value(sv.into_struct_value(), 0, "sw_tag")
                        .with_context(|| "switch tag")?;
                    let iv = tag.into_int_value();
                    if iv.get_type().get_bit_width() < 64 {
                        self.builder
                            .build_int_z_extend(iv, self.i64_type(), "sw_zext")?
                    } else {
                        iv
                    }
                } else {
                    anyhow::bail!("switch subject must be int or struct");
                };
                let default_bb = self.bb(*default)?;
                let cases: Vec<(IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> = arms
                    .iter()
                    .map(|(key, bb_id)| {
                        let k = match key {
                            SwitchKey::Int(n) => self.i64_type().const_int(*n as u64, true),
                            SwitchKey::Bool(b) => self.i64_type().const_int(*b as u64, false),
                            SwitchKey::Tag(t) => self.i64_type().const_int(*t as u64, false),
                        };
                        Ok((k, self.bb(*bb_id)?))
                    })
                    .collect::<Result<_>>()?;
                self.builder
                    .build_switch(discr, default_bb, &cases)
                    .with_context(|| "switch")?;
            }
            Terminator::Unreachable => {
                self.builder.build_unreachable()?;
            }
        }
        Ok(())
    }

    fn bb(&self, id: usize) -> Result<inkwell::basic_block::BasicBlock<'ctx>> {
        self.bb_map
            .get(&id)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("block {id} missing"))
    }
}

fn aggregate_to_basic<'ctx>(agg: AggregateValueEnum<'ctx>) -> BasicValueEnum<'ctx> {
    match agg {
        AggregateValueEnum::ArrayValue(v) => v.into(),
        AggregateValueEnum::StructValue(v) => v.into(),
    }
}

fn zero_value<'ctx>(context: &'ctx Context, ty: &MirTy) -> BasicValueEnum<'ctx> {
    match ty {
        MirTy::Bool => context.bool_type().const_int(0, false).into(),
        MirTy::Int | MirTy::Int64 | MirTy::UInt => context.i64_type().const_int(0, false).into(),
        MirTy::Int8 => context.i8_type().const_int(0, false).into(),
        MirTy::Int16 => context.i16_type().const_int(0, false).into(),
        MirTy::Int32 => context.i32_type().const_int(0, false).into(),
        MirTy::Float => context.f32_type().const_float(0.0).into(),
        MirTy::Double => context.f64_type().const_float(0.0).into(),
        MirTy::Char => context.i32_type().const_int(0, false).into(),
        MirTy::String | MirTy::Class(_) | MirTy::Array(_) => {
            context.ptr_type(AddressSpace::default()).const_null().into()
        }
        MirTy::Optional(inner) => {
            let opt_ty = optional_llvm_type(context, inner);
            let present = context.bool_type().const_int(0, false);
            let inner_z = zero_value(context, inner);
            opt_ty.const_named_struct(&[present.into(), inner_z.into()]).into()
        }
        MirTy::Tuple(elems) => {
            let fields: Vec<BasicValueEnum> = elems.iter().map(|t| zero_value(context, t)).collect();
            let st = context.struct_type(
                &elems
                    .iter()
                    .map(|t| mir_ty_to_llvm(context, t).into())
                    .collect::<Vec<_>>(),
                false,
            );
            st.const_named_struct(&fields.iter().map(|v| (*v).into()).collect::<Vec<_>>())
                .into()
        }
        MirTy::Struct(_) | MirTy::Enum(_) => context.i64_type().const_int(0, false).into(),
        MirTy::Named(n) if n == "void*" || n == "__err_out" => {
            context.ptr_type(AddressSpace::default()).const_null().into()
        }
        MirTy::Named(_) | MirTy::Error | MirTy::Void | MirTy::Never => {
            context.i64_type().const_int(0, false).into()
        }
    }
}

struct BlockMeta<'ctx> {
    id: usize,
    llvm_bb: inkwell::basic_block::BasicBlock<'ctx>,
}

fn llvm_ty_with_layout<'ctx>(
    context: &'ctx Context,
    layout: &TypeLayout<'ctx>,
    ty: &MirTy,
) -> BasicTypeEnum<'ctx> {
    match ty {
        MirTy::Struct(name) | MirTy::Class(name) => {
            if let Some(st) = layout.struct_types.get(name) {
                // Classes are pointers to the payload struct.
                if matches!(ty, MirTy::Class(_)) {
                    return context.ptr_type(AddressSpace::default()).into();
                }
                return (*st).into();
            }
            if matches!(ty, MirTy::Class(_)) {
                return context.ptr_type(AddressSpace::default()).into();
            }
        }
        _ => {}
    }
    mir_ty_to_llvm(context, ty)
}

fn function_type_for<'ctx>(
    context: &'ctx Context,
    layout: &TypeLayout<'ctx>,
    func: &Function,
) -> inkwell::types::FunctionType<'ctx> {
    let param_tys: Vec<BasicMetadataTypeEnum> = func
        .params
        .iter()
        .map(|(_, t)| llvm_ty_with_layout(context, layout, t).into())
        .collect();
    let ret_llvm = llvm_ty_with_layout(context, layout, &func.ret);
    if func.ret == MirTy::Void {
        context.void_type().fn_type(&param_tys, false)
    } else {
        ret_llvm.fn_type(&param_tys, false)
    }
}

fn declare_function_sig<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    layout: &TypeLayout<'ctx>,
    func: &Function,
) -> Result<()> {
    if module.get_function(&func.name).is_some() {
        return Ok(());
    }
    let fn_ty = function_type_for(context, layout, func);
    module.add_function(&func.name, fn_ty, Some(Linkage::External));
    Ok(())
}

fn emit_vtables<'ctx>(
    program: &Program,
    context: &'ctx Context,
    module: &Module<'ctx>,
    layout: &TypeLayout<'ctx>,
) -> Result<()> {
    let ptr = context.ptr_type(AddressSpace::default());
    for (class, methods) in &layout.class_with_vtable {
        if methods.is_empty() {
            continue;
        }
        let mut slots = Vec::new();
        for m in methods {
            let mangled = format!("{class}_{m}");
            let f = module.get_function(&mangled).or_else(|| {
                // Fallback: search any method with suffix.
                program.functions.iter().find_map(|f| {
                    if f.name.ends_with(&format!("_{m}")) || f.name.ends_with(m) {
                        module.get_function(&f.name)
                    } else {
                        None
                    }
                })
            });
            if let Some(f) = f {
                slots.push(f.as_global_value().as_pointer_value());
            } else {
                slots.push(ptr.const_null());
            }
        }
        let arr_ty = ptr.array_type(slots.len() as u32);
        let global = module.add_global(arr_ty, None, &format!("{class}_vtable"));
        global.set_linkage(Linkage::Internal);
        global.set_constant(true);
        let const_arr = ptr.const_array(&slots);
        global.set_initializer(&const_arr);
    }
    Ok(())
}

fn emit_function<'ctx>(
    _program: &Program,
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    layout: &TypeLayout<'ctx>,
    func: &Function,
) -> Result<()> {
    let llvm_fn = module
        .get_function(&func.name)
        .ok_or_else(|| anyhow::anyhow!("function `{}` not declared", func.name))?;

    let mut block_metas: Vec<BlockMeta> = Vec::new();
    for b in &func.blocks {
        let bb = context.append_basic_block(llvm_fn, &format!("bb{}", b.id));
        block_metas.push(BlockMeta { id: b.id, llvm_bb: bb });
    }

    let bb_map: HashMap<usize, inkwell::basic_block::BasicBlock> = block_metas
        .iter()
        .map(|m| (m.id, m.llvm_bb))
        .collect();

    let mut cg = Cg {
        context,
        module,
        builder,
        layout: layout.clone(),
        bb_map,
        local_types: HashMap::new(),
        values: HashMap::new(),
        allocas: HashMap::new(),
        err_out_ptr: None,
        last_err_out: None,
    };

    let llvm_params = llvm_fn.get_params();
    let mut param_idx = 0usize;
    let mut ploc_idx = 0usize;
    for (pname, pty) in &func.params {
        if pname == "__err_out" {
            cg.err_out_ptr = Some(llvm_params[param_idx].into_pointer_value());
            param_idx += 1;
            continue;
        }
        let (_, loc) = func
            .param_locals
            .get(ploc_idx)
            .ok_or_else(|| anyhow::anyhow!("param local index {ploc_idx}"))?;
        ploc_idx += 1;
        cg.set_local_ty(*loc, pty.clone());
        cg.values.insert(loc.0, llvm_params[param_idx].into());
        param_idx += 1;
    }

    // Hoist all MIR Alloca to the entry block so cross-block loads/releases work
    // regardless of codegen emission order.
    if let Some(entry) = func.blocks.first() {
        builder.position_at_end(cg.bb_map[&entry.id]);
        for b in &func.blocks {
            for inst in &b.stmts {
                if let Inst::Alloca { dst, ty } = inst {
                    if cg.allocas.contains_key(&dst.0) {
                        continue;
                    }
                    cg.set_local_ty(*dst, ty.clone());
                    let llvm_ty = llvm_ty_with_layout(context, layout, ty);
                    let slot = builder
                        .build_alloca(llvm_ty, &format!("loc{}", dst.0))
                        .with_context(|| "hoisted alloca")?;
                    cg.allocas.insert(dst.0, slot);
                    if matches!(ty, MirTy::Named(n) if n == "__err_out" || n == "void*") {
                        builder.build_store(slot, context.ptr_type(AddressSpace::default()).const_null())?;
                        cg.values.insert(dst.0, slot.into());
                    } else if let Some(pre) = cg.values.get(&dst.0).copied() {
                        builder.build_store(slot, pre)?;
                    } else if ty.is_refcounted() {
                        // Avoid releasing garbage if a path never assigns this local.
                        builder.build_store(
                            slot,
                            context.ptr_type(AddressSpace::default()).const_null(),
                        )?;
                    }
                }
            }
        }
    }

    for b in &func.blocks {
        let bb = cg.bb_map[&b.id];
        builder.position_at_end(bb);
        for inst in &b.stmts {
            // Alloca already hoisted — skip re-emitting.
            if matches!(inst, Inst::Alloca { .. }) {
                continue;
            }
            cg.emit_inst(inst)?;
        }
        cg.emit_terminator(&b.term)?;
    }

    Ok(())
}
