<?php

function process_a(array $items): int {
    $total = 0;
    foreach ($items as $item) {
        if ($item > 0) {
            $total += $item;
        }
    }
    return $total;
}

function process_b(array $items): int {
    $total = 0;
    foreach ($items as $item) {
        if ($item > 0) {
            $total += $item;
        }
    }
    return $total;
}

function process_c(array $items): int {
    $total = 0;
    foreach ($items as $item) {
        if ($item > 0) {
            $total += $item;
        }
    }
    return $total;
}
