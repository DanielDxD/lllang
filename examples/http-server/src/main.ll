namespace HttpServerEx

import Std.IO

public func handle(path: String) -> String {
    if path == "/health" || path == "/health/" {
        return "ok"
    }
    if path == "/" {
        return "hello from lllang"
    }
    "404 Not Found"
}

public func main() {
    let port = 18080
    print("listening on :\(port)")
    HttpServer.serve(port, handle)
}
