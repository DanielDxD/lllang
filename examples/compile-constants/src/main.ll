namespace CompileConstants

import Std.IO

#declare $SQL_DIALECT "sqlite"

public func main() {
    print($OS)
    print($ARCH)
    print($SQL_DIALECT)
    #if $DEBUG {
        print("debug")
    } else {
        print("release")
    }
}
