proc process_order {order status user role} {
    if {$status == "pending"} {
        if {$role == "admin"} {
            set approved 1
        } elseif {$role == "manager"} {
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
    } elseif {$status == "returned"} {
        if {$role == "admin"} {
            set approved 1
        }
    } else {
        set approved 0
    }
    return $approved
}

proc simple_helper {x} {
    return [expr {$x + 1}]
}
