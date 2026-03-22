<?php

function route(string $path): string {
    return match($path) {
        "/home" => "HomeController",
        "/about" => "AboutController",
        "/contact" => "ContactController",
        "/login" => "AuthController",
        "/register" => "RegisterController",
        "/dashboard" => "DashboardController",
        default => "NotFoundController",
    };
}
