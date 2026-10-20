import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
  const config = vscode.workspace.getConfiguration("lllang");
  const serverPath = config.get<string>("serverPath", "lang");

  const serverOptions: ServerOptions = {
    run: {
      command: serverPath,
      args: ["lsp"],
      transport: TransportKind.stdio,
    },
    debug: {
      command: serverPath,
      args: ["lsp"],
      transport: TransportKind.stdio,
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "lllang" },
      { scheme: "file", language: "lllang-sbl" },
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{ll,sbl}"),
    },
  };

  client = new LanguageClient(
    "lllang",
    "LLLang Language Server",
    serverOptions,
    clientOptions
  );

  void client.start();
  context.subscriptions.push({
    dispose: () => {
      void client?.stop();
    },
  });
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
