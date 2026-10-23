namespace SqlMemory

import Std.IO
import Std.SQL

public async func main() {
    let connection = await SQLManager.open(":memory:")
    #defer {
        await connection.close()
        print("connection closed")
    }
    let result = await connection.execute("SELECT 1")
    print(result)
}
