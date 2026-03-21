public class ReportGenerator
{
    public object[] ProcessUserReport(object[] users)
    {
        var report = new System.Collections.Generic.List<object>();
        foreach (var user in users)
        {
            var entry = new System.Collections.Generic.Dictionary<string, object>();
            entry["id"] = user.GetHashCode();
            entry["name"] = user.ToString();
            entry["status"] = "active";
            entry["formatted"] = System.DateTime.Now.ToString("yyyy-MM-dd");
            entry["display"] = user.ToString() + " (user)";
            report.Add(entry);
        }
        return report.ToArray();
    }

    public object[] ProcessAdminReport(object[] admins)
    {
        var report = new System.Collections.Generic.List<object>();
        foreach (var admin in admins)
        {
            var entry = new System.Collections.Generic.Dictionary<string, object>();
            entry["id"] = admin.GetHashCode();
            entry["name"] = admin.ToString();
            entry["status"] = "active";
            entry["formatted"] = System.DateTime.Now.ToString("yyyy-MM-dd");
            entry["display"] = admin.ToString() + " (admin)";
            report.Add(entry);
        }
        return report.ToArray();
    }

    public object[] ProcessVendorReport(object[] vendors)
    {
        var report = new System.Collections.Generic.List<object>();
        foreach (var vendor in vendors)
        {
            var entry = new System.Collections.Generic.Dictionary<string, object>();
            entry["id"] = vendor.GetHashCode();
            entry["name"] = vendor.ToString();
            entry["status"] = "active";
            entry["formatted"] = System.DateTime.Now.ToString("yyyy-MM-dd");
            entry["display"] = vendor.ToString() + " (vendor)";
            report.Add(entry);
        }
        return report.ToArray();
    }
}
