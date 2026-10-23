namespace StdlibSurface

import Std.IO
import Std.Log
import Std.Crypto
import Std.Process
import Std.Math

public func main() {
    let text = "  Hello, LLLang!  "
    let trimmed = text.trim()
    let upper = trimmed.uppercased()
    let lower = trimmed.lowercased()
    let replaced = trimmed.replacing("LLLang", "World")
    let parts = "apple,banana,orange".split(",")
    let joined = parts.joined("|")

    print(upper)
    print(lower)
    print(replaced)
    print(joined)

    let absolute = abs(-42)
    let maximum = max(10, 20)
    let minimum = min(10, 20)
    let bounded = clamp(150, 0, 100)
    let root = sqrt(144.0)
    let power = pow(2.0, 10.0)
    let sine = sin(PI / 2.0)
    let logarithm = log10(1000.0)

    print("abs=\(absolute) max=\(maximum) min=\(minimum) clamp=\(bounded)")
    print("sqrt=\(root) pow=\(power) sin=\(sine) log10=\(logarithm)")

    let price = Decimal("19.90")
    let quantity = Decimal("3")
    let subtotal = Decimal.mul(price, quantity)
    let discount = Decimal.mul(subtotal, Decimal("0.10"))
    let total = Decimal.sub(subtotal, discount)
    print("decimal_total_cents=\(total)")

    let message = "Hello, LLLang!"
    let base64 = Base64.encode(message)
    let hexadecimal = Hex.encode(message)
    let digest = Hash.sha256(message)
    let token = SecureRandom.bytes(16)
    print("b64=\(base64)")
    print("hex=\(hexadecimal)")
    print("sha256=\(digest)")
    print("rand_ok=\(token.isEmpty)")

    Log.debug("stdlib-surface ready")
    Log.info("ok")

    let handle = Process.start("true")
    let code = Process.wait(handle)
    print("process_exit=\(code)")
}
