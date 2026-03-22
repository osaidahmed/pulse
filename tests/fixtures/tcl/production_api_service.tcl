proc dispatch_command {cmd} {
    switch $cmd {
        create  { return "creating" }
        read    { return "reading" }
        update  { return "updating" }
        delete  { return "deleting" }
        list    { return "listing" }
        search  { return "searching" }
        default { return "unknown" }
    }
}

proc process_event {data} {
    set a [lindex $data 0]
    set b [lindex $data 1]
    set c [lindex $data 2]
    set d [lindex $data 3]
    set e [lindex $data 4]
    set f [lindex $data 5]
    set g [lindex $data 6]
    set h [lindex $data 7]
    set sum [expr {$a + $b}]
    set sum [expr {$sum + $c + $d}]
    set sum [expr {$sum + $e + $f}]
    set sum [expr {$sum + $g + $h}]
    set result [expr {$sum * 2}]
    set result [expr {$result + 1}]
    return $result
}
