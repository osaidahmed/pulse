"""Functions with duplicated logic (future: duplication detection)."""


def process_user_report(users):
    report = []
    for user in users:
        entry = {}
        entry["id"] = user.id
        entry["name"] = user.name
        entry["email"] = user.email
        entry["status"] = "active" if user.active else "inactive"
        entry["formatted_date"] = user.created_at.strftime("%Y-%m-%d")
        entry["display_name"] = f"{user.name} ({user.email})"
        report.append(entry)
    return report


def process_admin_report(admins):
    report = []
    for admin in admins:
        entry = {}
        entry["id"] = admin.id
        entry["name"] = admin.name
        entry["email"] = admin.email
        entry["status"] = "active" if admin.active else "inactive"
        entry["formatted_date"] = admin.created_at.strftime("%Y-%m-%d")
        entry["display_name"] = f"{admin.name} ({admin.email})"
        report.append(entry)
    return report


def process_vendor_report(vendors):
    report = []
    for vendor in vendors:
        entry = {}
        entry["id"] = vendor.id
        entry["name"] = vendor.name
        entry["email"] = vendor.email
        entry["status"] = "active" if vendor.active else "inactive"
        entry["formatted_date"] = vendor.created_at.strftime("%Y-%m-%d")
        entry["display_name"] = f"{vendor.name} ({vendor.email})"
        report.append(entry)
    return report
