<?php

function bumpy(int $a, int $b, int $c, int $d): int {
    $result = 0;
    if ($a > 0) {
        if ($b > 0) {
            if ($c > 0) {
                $result += $a;
            }
        }
    }
    $result += 1;
    if ($b > 0) {
        if ($c > 0) {
            if ($d > 0) {
                $result += $b;
            }
        }
    }
    return $result;
}
