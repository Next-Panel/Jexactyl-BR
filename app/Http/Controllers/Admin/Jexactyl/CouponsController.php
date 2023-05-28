<?php

namespace Pterodactyl\Http\Controllers\Admin\Jexactyl;

use Carbon\Carbon;
use Illuminate\View\View;
use Pterodactyl\Models\Coupon;
use Illuminate\Http\RedirectResponse;
use Prologue\Alerts\AlertsMessageBag;
use Pterodactyl\Exceptions\DisplayException;
use Pterodactyl\Http\Controllers\Controller;
use Pterodactyl\Exceptions\Model\DataValidationException;
use Pterodactyl\Exceptions\Repository\RecordNotFoundException;
use Pterodactyl\Contracts\Repository\SettingsRepositoryInterface;
use Pterodactyl\Http\Requests\Admin\Jexactyl\Coupons\IndexFormRequest;
use Pterodactyl\Http\Requests\Admin\Jexactyl\Coupons\StoreFormRequest;

class CouponsController extends Controller
{
    public function __construct(private AlertsMessageBag $alert, private SettingsRepositoryInterface $settings)
    {
    }

    public function index(): View
    {
        return view('admin.jexactyl.coupons', [
            'coupons' => Coupon::all(),
            'enabled' => $this->settings->get('jexactyl::coupons:enabled'),
        ]);
    }

    /**
     * @throws DataValidationException
     * @throws RecordNotFoundException
     */
    public function update(IndexFormRequest $request): RedirectResponse
    {
        foreach ($request->normalize() as $key => $value) {
            $this->settings->set('jexactyl::coupons:' . $key, $value);
        }

        $this->alert->success('O sistema de cupons foi atualizado com sucesso.')->flash();

        return redirect()->route('admin.jexactyl.coupons');
    }

    /**
     * @throws DisplayException
     */
    public function store(StoreFormRequest $request): RedirectResponse
    {
        if ($request->input('expires')) {
            $expires_at = Carbon::now()->addHours($request->input('expires'));
        } else {
            $expires_at = null;
        }

        if (Coupon::where(['code' => $request->input('code')])->exists()) {
            throw new DisplayException('Você não pode criar um cupom com um código já existente.');
        }

        Coupon::query()->insert([
            'expires' => $expires_at,
            'created_at' => Carbon::now(),
            'code' => $request->input('code'),
            'uses' => $request->input('uses'),
            'cr_amount' => $request->input('credits'),
        ]);

        $this->alert->success('Criou com sucesso um cupom.')->flash();

        return redirect()->route('admin.jexactyl.coupons');
    }
}
