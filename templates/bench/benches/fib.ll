namespace {{Name}}Bench

func twice(n: Int) -> Int {
    n + n
}

@Bench
func twice10() {
    let _x = twice(10)
}
