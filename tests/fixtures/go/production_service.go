package main

import (
	"errors"
	"fmt"
	"strings"
	"time"
)

type PaymentService struct {
	gateway    string
	db         string
	cache      string
	logger     string
	notifier   string
	config     map[string]string
	retryLimit int
}

func NewPaymentService(gateway, db, cache, logger, notifier, apiKey string, retryLimit int) *PaymentService {
	cfg := map[string]string{"apiKey": apiKey}
	return &PaymentService{
		gateway:    gateway,
		db:         db,
		cache:      cache,
		logger:     logger,
		notifier:   notifier,
		config:     cfg,
		retryLimit: retryLimit,
	}
}

func (ps *PaymentService) PaymentServiceProcessPayment(amount float64, currency string, method string, userID string) (string, error) {
	if amount <= 0 {
		return "", errors.New("invalid amount")
	}
	if currency == "" {
		return "", errors.New("missing currency")
	}

	if method == "credit_card" {
		if amount > 10000 {
			return "", errors.New("credit card limit exceeded")
		}
		if currency != "USD" && currency != "EUR" {
			return "", errors.New("unsupported currency for credit card")
		}
		return fmt.Sprintf("cc_%s_%d", userID, time.Now().UnixNano()), nil
	} else if method == "bank_transfer" {
		if amount < 100 {
			return "", errors.New("minimum amount for bank transfer is 100")
		}
		return fmt.Sprintf("bt_%s_%d", userID, time.Now().UnixNano()), nil
	} else if method == "crypto" {
		if currency != "BTC" && currency != "ETH" {
			return "", errors.New("unsupported crypto currency")
		}
		return fmt.Sprintf("cr_%s_%d", userID, time.Now().UnixNano()), nil
	} else if method == "wallet" {
		return fmt.Sprintf("wl_%s_%d", userID, time.Now().UnixNano()), nil
	} else {
		return "", errors.New("unknown payment method")
	}
}

func (ps *PaymentService) PaymentServiceGetStatus(txID string) string {
	if strings.HasPrefix(txID, "cc_") {
		return "credit_card_pending"
	}
	return "unknown"
}

func (ps *PaymentService) PaymentServiceRefund(txID string) error {
	if txID == "" {
		return errors.New("empty transaction ID")
	}
	return nil
}

func (ps *PaymentService) PaymentServiceConfig(key string) string {
	return ps.config[key]
}
