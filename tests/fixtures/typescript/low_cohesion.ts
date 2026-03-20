class KitchenSink {
    private users: any[] = [];
    private orders: any[] = [];
    private logs: any[] = [];
    private config: any = {};

    addUser(user: any) { this.users.push(user); }
    removeUser(id: number) { this.users = this.users.filter(u => u.id !== id); }
    getUser(id: number) { return this.users.find(u => u.id === id); }

    createOrder(order: any) { this.orders.push(order); }
    cancelOrder(id: number) { this.orders = this.orders.filter(o => o.id !== id); }
    getOrder(id: number) { return this.orders.find(o => o.id === id); }

    logEvent(event: any) { this.logs.push(event); }
    getLogs() { return this.logs; }
    clearLogs() { this.logs = []; }

    setConfig(key: string, value: any) { this.config[key] = value; }
    getConfig(key: string) { return this.config[key]; }
}
