<?php

namespace Jexactyl\Http\Controllers\Admin\Jexactyl;

use Illuminate\View\View;
use Illuminate\Http\RedirectResponse;
use Prologue\Alerts\AlertsMessageBag;
use Illuminate\Contracts\Console\Kernel;
use Jexactyl\Http\Controllers\Controller;
use Illuminate\View\Factory as ViewFactory;
use Jexactyl\Contracts\Repository\SettingsRepositoryInterface;
use Jexactyl\Http\Requests\Admin\Jexactyl\AdvancedFormRequest;
use Illuminate\Contracts\Config\Repository as ConfigRepository;

class AdvancedController extends Controller
{
    /**
     * AdvancedController constructor.
     */
    public function __construct(
        private AlertsMessageBag $alert,
        private ConfigRepository $config,
        private Kernel $kernel,
        private SettingsRepositoryInterface $settings,
        private ViewFactory $view
    ) {
    }

    /**
     * Render advanced Panel settings UI.
     */
    public function index(): View
    {
        $warning = false;

        if (
            $this->config->get('recaptcha._shipped_secret_key') == $this->config->get('recaptcha.secret_key')
            || $this->config->get('recaptcha._shipped_website_key') == $this->config->get('recaptcha.website_key')
        ) {
            $warning = true;
        }

        return $this->view->make('admin.jexactyl.advanced', [
            'warning' => $warning,
            'logo' => $this->settings->get('settings::app:logo', 'https://avatars.githubusercontent.com/u/91636558'),
        ]);
    }

    /**
     * @throws \Jexactyl\Exceptions\Model\DataValidationException
     * @throws \Jexactyl\Exceptions\Repository\RecordNotFoundException
     */
    public function update(AdvancedFormRequest $request): RedirectResponse
    {
        foreach ($request->normalize() as $key => $value) {
            $this->settings->set('settings::' . $key, $value);
        }

        $this->kernel->call('queue:restart');
        $this->alert->success('As configurações avançadas foram atualizadas com sucesso e o trabalhador da Queue foi reiniciado para aplicar essas alterações.')->flash();

        return redirect()->route('admin.jexactyl.advanced');
    }
}
