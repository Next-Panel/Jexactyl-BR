<?php

namespace Jexactyl\Console\Commands\Schedule;

use Carbon\Carbon;
use Jexactyl\Models\Coupon;
use Illuminate\Console\Command;

class CouponCommand extends Command
{
    protected $signature = 'p:schedule:coupon';
    protected $description = 'Processar expirações de cupons.';

    public function handle(): void
    {
        $this->line('Início da verificação dos cupons vencidos.');
        $coupons = Coupon::query()->get();
        foreach ($coupons as $coupon) {
            $carbon = new Carbon($coupon->expires);
            $expires = $carbon->timestamp;
            if (Carbon::now()->timestamp >= $expires) {
                $coupon->update(['expired' => true]);
                $this->line('Cupom #' . $coupon->id . ' foi definido como expirado.');
            }
        }
        $this->line('Verificação completa para cupons vencidos.');
    }
}
