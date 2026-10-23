namespace Inheritance

import Std.IO

public class Animal(name: String) {
    public virtual func speak() -> String {
        "\(name) makes a sound"
    }
}

public class Dog(name: String, breed: String): Animal {
    public override func speak() -> String {
        "\(name) the \(breed) barks"
    }

    public func speakLikeParent() -> String {
        super.speak()
    }
}

public func main() {
    let d = Dog(name: "Rex", breed: "Lab")
    print(d.speak())
    print(d.speakLikeParent())
}
