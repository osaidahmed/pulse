<?php

class Kitchen {
    public function bake(): string {
        return $this->oven . " baking";
    }

    public function checkOven(): int {
        return $this->oven_temp;
    }

    public function wash(): string {
        return $this->sink . " washing";
    }

    public function drySink(): int {
        return $this->sink_level;
    }

    public function sweep(): string {
        return $this->broom . " sweeping";
    }

    public function storeBroom(): int {
        return $this->broom_closet;
    }
}
