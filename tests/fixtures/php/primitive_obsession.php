<?php

function create_user(string $name, string $email, int $age, bool $active, string $role): array {
    return compact('name', 'email', 'age', 'active', 'role');
}

function update_record(int $id, string $field, string $value, int $version): bool {
    return true;
}
