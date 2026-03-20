function processUserReport(users) {
    const report = [];
    for (const user of users) {
        const entry = {};
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

function processAdminReport(admins) {
    const report = [];
    for (const admin of admins) {
        const entry = {};
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

function processVendorReport(vendors) {
    const report = [];
    for (const vendor of vendors) {
        const entry = {};
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
