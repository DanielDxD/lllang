use lllang_manifest::{load_package, load_solution};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn load_hello_package() {
    let path = repo_root().join("examples/hello/Package.ll");
    let manifest = load_package(&path).expect("hello Package.ll");
    assert_eq!(manifest.name, "hello");
    assert!(!manifest.products.is_empty());
    assert!(!manifest.targets.is_empty());
}

#[test]
fn load_lib_geometry_package() {
    let path = repo_root().join("examples/lib-geometry/Package.ll");
    let manifest = load_package(&path).expect("lib-geometry Package.ll");
    assert_eq!(manifest.name, "lib.geometry");
    assert!(!manifest.products.is_empty());
    assert!(!manifest.targets.is_empty());
}

#[test]
fn load_shop_solution() {
    let path = repo_root().join("examples/shop-solution/Solution.ll");
    let solution = load_solution(&path).expect("shop Solution.ll");
    assert_eq!(solution.name, "Shop");
    assert!(!solution.projects.is_empty());
}
