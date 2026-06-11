proc flat_calls {} {
    set a 1
    set b 2
    set c [expr {$a + $b}]
    return $c
}

proc pick_branch {x} {
    if {$x > 10} {
        return 3
    } elseif {$x > 5} {
        return 2
    } else {
        return 1
    }
}

proc nested_guard {a b c} {
    if {$a > 0} {
        if {$b > 0} {
            if {$c > 0} {
                return 1
            }
        }
    }
    return 0
}

proc loop_filter {n} {
    set total 0
    for {set i 0} {$i < $n} {incr i} {
        if {$i % 2 == 0} {
            set total [expr {$total + $i}]
        }
    }
    return $total
}

proc wide_params {a b c d e f} {
    return [expr {$a + $b + $c + $d + $e + $f}]
}

proc bool_blend {a b c d} {
    if {($a > 0 && $b > 0) || ($c > 0 && $d > 0)} {
        return 1
    }
    return 0
}
