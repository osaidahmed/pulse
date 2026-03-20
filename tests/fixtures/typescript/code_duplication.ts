function processUserReport(users: any[]): any[] {
    const report: any[] = [];
    for (const user of users) {
        const entry: any = {};
        entry.id = user.id;
        entry.name = user.name;
        entry.email = user.email;
        entry.status = user.active ? "active" : "inactive";
        entry.formattedDate = user.createdAt;
        entry.displayName = `${user.name} (${user.email})`;
        report.push(entry);
    }
    return report;
}

function processAdminReport(admins: any[]): any[] {
    const report: any[] = [];
    for (const admin of admins) {
        const entry: any = {};
        entry.id = admin.id;
        entry.name = admin.name;
        entry.email = admin.email;
        entry.status = admin.active ? "active" : "inactive";
        entry.formattedDate = admin.createdAt;
        entry.displayName = `${admin.name} (${admin.email})`;
        report.push(entry);
    }
    return report;
}

function processVendorReport(vendors: any[]): any[] {
    const report: any[] = [];
    for (const vendor of vendors) {
        const entry: any = {};
        entry.id = vendor.id;
        entry.name = vendor.name;
        entry.email = vendor.email;
        entry.status = vendor.active ? "active" : "inactive";
        entry.formattedDate = vendor.createdAt;
        entry.displayName = `${vendor.name} (${vendor.email})`;
        report.push(entry);
    }
    return report;
}
