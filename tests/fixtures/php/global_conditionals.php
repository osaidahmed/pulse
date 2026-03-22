<?php

if (PHP_SAPI === 'cli') {
    echo "Running from CLI\n";
}

if (defined('DEBUG')) {
    error_reporting(E_ALL);
}
