namespace HmacSmoke
import Std.IO
import Std.Crypto
public func main() {
    let d = HMAC.sha256("key", "message")
    print(d)
}
