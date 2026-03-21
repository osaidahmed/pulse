package main

func DeeplyNested(data [][][]int) bool {
	for _, group := range data {
		if len(group) > 0 {
			for _, items := range group {
				if len(items) > 0 {
					for _, val := range items {
						if val > 0 {
							process(val)
						}
					}
				}
			}
		}
	}
	return true
}

func ModeratelyNested(data []int) bool {
	for _, item := range data {
		if item > 0 {
			process(item)
		}
	}
	return true
}

func process(x int) {}
