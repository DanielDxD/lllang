namespace {{Name}}

import Std.IO
import Std.FS

public func handle(path: String) -> String {
    if path == "/health" || path == "/health/" {
        return healthBody()
    }
    if path == "/" || path == "/index.html" {
        if File.exists("assets/index.html") {
            return File.readText("assets/index.html")
        }
        return "<html><body>{{Name}}</body></html>"
    }
    "404 Not Found"
}

public func main() {
    let port = 8080
    print("{{Name}} listening on :\(port)")
    HttpServer.serve(port, handle)
}
