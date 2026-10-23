namespace {{Name}}

public func twice(n: Int) -> Int {
    n + n
}

public func fib(n: Int) -> Int {
    if n < 2 {
        return n
    }
    fib(n - 1) + fib(n - 2)
}
