namespace RegexSmoke

import Std.IO
import Std.Text

public func main() {
    let ok = Regex.isMatch("[0-9]+", "abc123xyz")
    let no = Regex.isMatch("^hello$", "world")
    let viaString = "dev@example.com".matches(".*@.*")
    print("ok=\(ok) no=\(no) via=\(viaString)")
}
