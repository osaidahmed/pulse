public class DataAccess
{
    public object GetActiveUsers(object db)
    {
        string query = @"
            SELECT u.id,
                   u.first_name,
                   u.last_name,
                   u.email,
                   u.phone,
                   u.created_at,
                   u.updated_at,
                   u.last_login,
                   u.status,
                   u.role,
                   u.department,
                   u.manager_id,
                   u.location,
                   u.timezone,
                   u.language
            FROM users u
            JOIN departments d ON u.department_id = d.id
            WHERE u.status = 'active'
              AND u.deleted_at IS NULL
              AND u.last_login > NOW() - INTERVAL '90 days'
            ORDER BY u.last_name, u.first_name
        ";
        return query;
    }

    public object SimpleQuery(object db)
    {
        return "SELECT id FROM users";
    }
}
