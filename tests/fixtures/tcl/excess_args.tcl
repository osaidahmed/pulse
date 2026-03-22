proc create_user {name email age role dept city country phone} {
    set user [dict create]
    dict set user name $name
    dict set user email $email
    dict set user age $age
    dict set user role $role
    dict set user dept $dept
    dict set user city $city
    dict set user country $country
    dict set user phone $phone
    return $user
}

proc simple_func {a b} {
    return [expr {$a + $b}]
}

namespace eval UserService {
    proc init {db cache logger mailer validator config} {
        variable db_conn $db
        variable cache_conn $cache
        variable logger_inst $logger
        variable mailer_inst $mailer
        variable validator_inst $validator
        variable config_inst $config
    }
}
