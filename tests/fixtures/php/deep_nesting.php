<?php

function deeply_nested(int $a, int $b, int $c, int $d, int $e): int {
    if ($a > 0) {
        if ($b > 0) {
            for ($i = 0; $i < $c; $i++) {
                if ($d > 0) {
                    while ($e > 0) {
                        $e--;
                    }
                }
            }
        }
    }
    return $e;
}
