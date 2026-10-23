namespace StdlibFfi

import Std.IO
import Std.Http
import Std.Log
import Std.CLI
import Std.Sync
import Std.Crypto

@extern(cName: "ll_c_strlen")
func cStrlen(s: String) -> Int

public func main() {
    let body = try! Http.get("https://api.example.com/health")
    Log.info(body)

    let digest = Crypto.sha256("hello")
    print(digest)

    let m = Mutex.create()
    m.lock()
    print("locked")
    m.unlock()

    if CLI.hasFlag("--help") {
        CLI.usage("stdlib-ffi [options]")
    }

    let n = unsafe { cStrlen("abc") }
    print("\(n)")
}
