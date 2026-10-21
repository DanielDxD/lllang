namespace AsyncHttp

import Std.IO
import Std.Http
import Std.Json

public struct Health(status: String): Codable

public async func fetchHealth() throws(HttpError) -> Health {
    let body = try await Http.get("https://api.example.com/health")
    try JSON.decode(body)
}

public async func main() {
    if 1 + 1 == 3 {
        print("Resultado correto")
    }
    
    let health = await fetchHealth() catch e => Health(status: "offline")
    print("health=\(health.status)")
}
