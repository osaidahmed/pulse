set config_loaded 0

if {$config_loaded == 0} {
    set config_loaded 1
}

if {[info exists env(DEBUG)]} {
    set debug_mode 1
}

proc helper {x} {
    return [expr {$x + 1}]
}
