<?php

namespace Pterodactyl\Http\Controllers\Admin\Jexactyl;

use Illuminate\View\View;
use Pterodactyl\Http\Controllers\Controller;
use Illuminate\View\Factory as ViewFactory;

class UpgradeController extends Controller
{
    public function __construct(private ViewFactory $view)
    {
        //
    }

    /**
     * Render the v4 upgrade interface.
     */
    public function index(): View
    {
        return $this->view->make('admin.jexactyl.upgrade', [
            'php_version' => phpversion(),
            'total_memory' => $this->getTotalSystemMemory(),
            'storage_available' => $this->getTotalSystemDisk(),
        ]);
    }

    /**
     * Get the total amount of system memory of this host.
     */
    private function getTotalSystemMemory(): float
    {
        $memInfo = file_get_contents('/proc/meminfo');

        if (preg_match('/^MemTotal:\s+(\d+)\skB$/m', $memInfo, $matches)) {
            $memKB = (int) $matches[1];
            return round($memKB / 1024 / 1024, 2);
        }
    }

    /**
     * Ensure the disk has enough space to continue.
     */
    private function getTotalSystemDisk(): bool
    {
        if (disk_free_space('/') < 1024 * 1024 * 4096) {
            return false;
        }

        return true;
    }
}
