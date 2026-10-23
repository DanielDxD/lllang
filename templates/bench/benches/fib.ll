namespace {{Name}}Bench

import {{Name}}

@Bench
public func benchTwice() {
    let _x = twice(10)
}

@Bench
public func benchFib() {
    let _x = fib(15)
}
