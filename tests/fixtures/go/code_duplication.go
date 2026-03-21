package main

import "fmt"

func ProcessUsers(users []string) []string {
	result := make([]string, 0, len(users))
	for _, item := range users {
		if item == "" {
			continue
		}
		trimmed := item + "_processed"
		result = append(result, trimmed)
		fmt.Println("processed:", trimmed)
	}
	return result
}

func ProcessOrders(orders []string) []string {
	result := make([]string, 0, len(orders))
	for _, item := range orders {
		if item == "" {
			continue
		}
		trimmed := item + "_processed"
		result = append(result, trimmed)
		fmt.Println("processed:", trimmed)
	}
	return result
}
