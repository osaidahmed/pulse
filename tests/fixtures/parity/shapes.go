package shapes

func flatCalls() int {
	a := 1
	b := 2
	c := a + b
	return c
}

func pickBranch(x int) int {
	if x > 10 {
		return 3
	} else if x > 5 {
		return 2
	} else {
		return 1
	}
}

func nestedGuard(a int, b int, c int) int {
	if a > 0 {
		if b > 0 {
			if c > 0 {
				return 1
			}
		}
	}
	return 0
}

func loopFilter(n int) int {
	total := 0
	for i := 0; i < n; i++ {
		if i%2 == 0 {
			total += i
		}
	}
	return total
}

func wideParams(a int, b int, c int, d int, e int, f int) int {
	return a + b + c + d + e + f
}

func boolBlend(a int, b int, c int, d int) int {
	if (a > 0 && b > 0) || (c > 0 && d > 0) {
		return 1
	}
	return 0
}
