proc deeply_nested {a b c d e} {
    if {$a > 0} {
        if {$b > 0} {
            if {$c > 0} {
                if {$d > 0} {
                    if {$e > 0} {
                        return 1
                    }
                }
            }
        }
    }
    return 0
}

proc moderately_nested {x y} {
    if {$x > 0} {
        if {$y > 0} {
            return 1
        }
    }
    return 0
}
