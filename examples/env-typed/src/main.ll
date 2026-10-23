namespace EnvTyped

import Std.IO
import Std.Env

@loadEnv
public func main() {
    let port = Env.integer("PORT", 3000)
    let databaseUrl = Env.string("DATABASE_URL", "postgres://localhost/app")
    let debug = Env.bool("DEBUG", false)
    let rate = Env.double("RATE", 1.5)
    let price = Env.decimal("PRICE", Decimal("9.99"))

    print("port=\(port)")
    print("db=\(databaseUrl)")
    print("debug=\(debug)")
    print("rate=\(rate)")
    print("price=\(price)")

    let maybeHost = Env.string("HOST") 
    print(maybeHost ?? "no-host")

    let maybePort = Env.integer("MISSING_PORT")
    print("missingPort=\(maybePort ?? -1)")

    // Environment remains an alias of Env.
    let alias = Environment.get("PATH")
    print(alias ?? "no-path")
}
