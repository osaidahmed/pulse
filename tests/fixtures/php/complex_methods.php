<?php

function process_order(int $status, int $priority, float $amount, float $discount, float $tax): float {
    $result = 0;
    if ($status == 1) {
        if ($priority > 5) {
            $result = $amount * 2;
        } elseif ($priority > 3) {
            $result = $amount;
        } else {
            $result = $amount / 2;
        }
    } elseif ($status == 2) {
        if ($discount > 0) {
            $result = $amount - $discount;
        }
    } elseif ($status == 3) {
        switch ($priority) {
            case 1:
                $result = $amount + $tax;
                break;
            case 2:
                $result = $amount - $tax;
                break;
            case 3:
                $result = $amount;
                break;
        }
    }
    if ($result < 0) {
        $result = 0;
    }
    return $result;
}

function simple_helper(int $x): int {
    return $x + 1;
}
