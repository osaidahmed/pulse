function getActiveUsers(db) {
    const query = `
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
        ORDER BY u.last_name
    `;
    return db.execute(query);
}

function simpleQuery(db) {
    return db.execute("SELECT id FROM users");
}
