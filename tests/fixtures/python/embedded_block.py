"""Function with large embedded string content (SQL, templates, etc.)."""


def get_active_users(db):
    query = """
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
    """
    return db.execute(query)


def simple_query(db):
    return db.execute("SELECT id FROM users")
