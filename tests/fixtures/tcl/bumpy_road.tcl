proc validate {data} {
    if {[llength $data] > 0} {
        if {[lindex $data 0] > 0} {
            if {[lindex $data 0] > 10} {
                set x 1
            }
        }
    }
    set gap 1
    if {[llength $data] > 5} {
        if {[lindex $data 5] > 0} {
            if {[lindex $data 5] > 10} {
                set y 2
            }
        }
    }
    set gap2 2
    if {[llength $data] > 10} {
        if {[lindex $data 10] > 0} {
            if {[lindex $data 10] > 10} {
                set z 3
            }
        }
    }
    return 0
}
