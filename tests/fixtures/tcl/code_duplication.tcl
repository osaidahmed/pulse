proc process_alpha {data} {
    set result 0
    foreach item $data {
        if {$item > 100} {
            set result [expr {$result + 2}]
        } else {
            set result [expr {$result + 1}]
        }
    }
    set result [expr {$result * 2}]
    return $result
}

proc process_beta {data} {
    set result 0
    foreach item $data {
        if {$item > 100} {
            set result [expr {$result + 2}]
        } else {
            set result [expr {$result + 1}]
        }
    }
    set result [expr {$result * 2}]
    return $result
}

proc process_gamma {data} {
    set result 0
    foreach item $data {
        if {$item > 100} {
            set result [expr {$result + 2}]
        } else {
            set result [expr {$result + 1}]
        }
    }
    set result [expr {$result * 2}]
    return $result
}
