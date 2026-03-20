"""Test file with test-specific smells (future: assertion blocks)."""

import unittest


class TestUserCreation(unittest.TestCase):
    def test_full_user_validation(self):
        user = create_user("John", "Doe", "john@example.com")
        assert user.first_name == "John"
        assert user.last_name == "Doe"
        assert user.email == "john@example.com"
        assert user.active is True
        assert user.verified is False
        assert user.role == "user"
        assert user.permissions == []
        assert user.created_at is not None
        assert user.updated_at is not None
        assert user.last_login is None
        assert user.avatar_url is None
        assert user.bio == ""
        assert user.settings == {}
        assert user.notifications_enabled is True

    def test_admin_user_validation(self):
        admin = create_user("Admin", "User", "admin@example.com", role="admin")
        assert admin.first_name == "Admin"
        assert admin.last_name == "User"
        assert admin.email == "admin@example.com"
        assert admin.active is True
        assert admin.verified is False
        assert admin.role == "admin"
        assert admin.permissions == []
        assert admin.created_at is not None
        assert admin.updated_at is not None
        assert admin.last_login is None
        assert admin.avatar_url is None
        assert admin.bio == ""
        assert admin.settings == {}
        assert admin.notifications_enabled is True


def create_user(first, last, email, role="user"):
    pass
