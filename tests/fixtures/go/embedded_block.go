package main

import "fmt"

func RunMigration() string {
	query := `
		CREATE TABLE IF NOT EXISTS users (
			id         SERIAL PRIMARY KEY,
			username   VARCHAR(255) NOT NULL UNIQUE,
			email      VARCHAR(255) NOT NULL UNIQUE,
			password   VARCHAR(255) NOT NULL,
			first_name VARCHAR(100),
			last_name  VARCHAR(100),
			role       VARCHAR(50) DEFAULT 'user',
			active     BOOLEAN DEFAULT TRUE,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			last_login TIMESTAMP,
			avatar_url TEXT,
			bio        TEXT,
			settings   JSONB DEFAULT '{}',
			verified   BOOLEAN DEFAULT FALSE,
			deleted_at TIMESTAMP,
			org_id     INTEGER REFERENCES organizations(id)
		);
		CREATE INDEX idx_users_email ON users(email);
		CREATE INDEX idx_users_username ON users(username);
		CREATE INDEX idx_users_org ON users(org_id);
	`
	fmt.Println("Running migration")
	return query
}
