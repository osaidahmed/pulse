package main

type Manager struct {
	users  []string
	orders []int
	logs   []string
	config map[string]string
}

func NewManager() *Manager {
	return &Manager{
		config: make(map[string]string),
	}
}

func (m *Manager) AddUser(u string) {
	m.users = append(m.users, u)
}

func (m *Manager) GetUsers() []string {
	return m.users
}

func (m *Manager) AddOrder(o int) {
	m.orders = append(m.orders, o)
}

func (m *Manager) TotalOrders() int {
	total := 0
	for _, o := range m.orders {
		total += o
	}
	return total
}

func (m *Manager) AddLog(entry string) {
	m.logs = append(m.logs, entry)
}

func (m *Manager) GetLogs() []string {
	return m.logs
}

func (m *Manager) SetConfig(key, value string) {
	m.config[key] = value
}

func (m *Manager) GetConfig(key string) string {
	return m.config[key]
}
