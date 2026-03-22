namespace eval OrderService {
    variable db
    variable cache
    variable logger

    proc init {db_conn cache_conn log_inst mailer validator config} {
        variable db $db_conn
        variable cache $cache_conn
        variable logger $log_inst
    }

    proc process_order {order_id status user role priority} {
        variable db
        variable cache
        if {$status == "pending"} {
            if {$role == "admin"} {
                if {$priority > 5} {
                    set approved 1
                } else {
                    set approved 0
                }
            } elseif {$role == "manager" && $priority > 3} {
                set approved 1
            } else {
                set approved 0
            }
        } elseif {$status == "shipped"} {
            if {$user != ""} {
                set approved 1
            }
        } elseif {$status == "cancelled"} {
            set approved 0
        } else {
            set approved 0
        }
        return $approved
    }

    proc validate_order {type} {
        switch $type {
            standard { return 1 }
            express  { return 2 }
            premium  { return 3 }
            bulk     { return 4 }
            trial    { return 5 }
            default  { return 0 }
        }
    }

    proc generate_report {items} {
        set total 0
        foreach item $items {
            if {$item > 0} {
                set total [expr {$total + $item}]
            }
        }
        return $total
    }

    proc get_db {} {
        variable db
        return $db
    }

    proc get_cache {} {
        variable cache
        return $cache
    }
}
