package main

func ProcessRecords(records []map[string]interface{}) []string {
	var results []string

	for _, record := range records {
		name, ok := record["name"].(string)
		if ok {
			if len(name) > 0 {
				age, hasAge := record["age"].(int)
				if hasAge && age > 18 {
					results = append(results, name)
				}
			}
		}

		email, ok := record["email"].(string)
		if ok {
			if len(email) > 5 {
				verified, isVerified := record["verified"].(bool)
				if isVerified && verified {
					results = append(results, email)
				}
			}
		}

		role, ok := record["role"].(string)
		if ok {
			if role == "admin" {
				active, isActive := record["active"].(bool)
				if isActive && active {
					results = append(results, role)
				}
			}
		}
	}

	return results
}
