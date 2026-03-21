package main

func CreateUser(first, last, email, phone, addr, city, state, zip string) string {
	return first + " " + last
}

func SimpleFunc(a, b int) int {
	return a + b
}

type UserService struct {
	db        string
	cache     string
	logger    string
	mailer    string
	validator string
	scheduler string
}

func NewUserService(db, cache, logger, mailer, validator, scheduler string) *UserService {
	return &UserService{
		db:        db,
		cache:     cache,
		logger:    logger,
		mailer:    mailer,
		validator: validator,
		scheduler: scheduler,
	}
}

func (s *UserService) GetUser(userID string) string {
	return s.db
}
