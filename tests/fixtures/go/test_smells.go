package main

import "fmt"

func assertEqual(a, b interface{}) {
	if a != b {
		panic(fmt.Sprintf("assertion failed: %v != %v", a, b))
	}
}

func assertTrue(v bool) {
	if !v {
		panic("assertion failed: expected true")
	}
}

func test_UserRegistration() {
	assertEqual(Add(1, 2), 3)
	assertEqual(Add(0, 0), 0)
	assertEqual(Add(-1, 1), 0)
	assertTrue(Add(2, 3) == 5)
	assertEqual(Add(10, 20), 30)
	assertTrue(Add(100, 200) == 300)
	assertEqual(Add(-5, -3), -8)
	assertEqual(Add(1, -1), 0)
	assertTrue(Add(7, 7) == 14)
	assertEqual(Add(999, 1), 1000)
	assertTrue(Add(50, 50) == 100)
	assertEqual(Add(0, 1), 1)
}

func test_OrderProcessing() {
	assertEqual(Max(1, 2), 2)
	assertEqual(Max(5, 3), 5)
	assertEqual(Max(0, 0), 0)
	assertTrue(Max(10, 5) == 10)
	assertEqual(Max(-1, -2), -1)
	assertTrue(Max(100, 99) == 100)
	assertEqual(Max(7, 7), 7)
	assertEqual(Max(-5, 0), 0)
	assertTrue(Max(1000, 999) == 1000)
	assertEqual(Max(42, 43), 43)
	assertTrue(Max(0, 1) == 1)
	assertEqual(Max(3, 3), 3)
}
