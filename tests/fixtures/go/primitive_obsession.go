package main

import "fmt"

func RegisterEmployee(
	firstName string,
	lastName string,
	age int,
	salary float64,
	department string,
	isActive bool,
	level int,
) string {
	if age < 18 {
		return "too young"
	}
	if salary < 0 {
		return "invalid salary"
	}
	return fmt.Sprintf("%s %s, dept=%s, level=%d", firstName, lastName, department, level)
}

func CalculateShipping(
	weight float64,
	length float64,
	width float64,
	height float64,
	zipCode string,
	express bool,
) float64 {
	volume := length * width * height
	base := weight * 0.5
	if volume > 1000 {
		base += 10.0
	}
	if express {
		base *= 2.0
	}
	_ = zipCode
	return base
}
