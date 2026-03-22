namespace eval Service {
    variable db_host
    variable db_port
    variable cache_host
    variable cache_port
    variable log_level
    variable log_file
    variable api_key
    variable api_url

    proc get_db_host {} {
        variable db_host
        return $db_host
    }

    proc get_db_port {} {
        variable db_port
        return $db_port
    }

    proc get_cache_host {} {
        variable cache_host
        return $cache_host
    }

    proc get_cache_port {} {
        variable cache_port
        return $cache_port
    }

    proc get_log_level {} {
        variable log_level
        return $log_level
    }

    proc get_log_file {} {
        variable log_file
        return $log_file
    }

    proc get_api_key {} {
        variable api_key
        return $api_key
    }

    proc get_api_url {} {
        variable api_url
        return $api_url
    }
}
