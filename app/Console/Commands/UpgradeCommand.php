<?php

namespace Pterodactyl\Console\Commands;

use Illuminate\Console\Command;
use Pterodactyl\Console\Kernel;
use Symfony\Component\Process\Process;
use Symfony\Component\Console\Helper\ProgressBar;

class UpgradeCommand extends Command
{
    protected const DEFAULT_URL = 'https://github.com/Next-Panel/Jexactyl-BR/releases/%s/panel.tar.gz';

    protected $signature = 'p:upgrade
        {--user= : O usuário sob o qual o PHP é executado. Todos os arquivos serão propriedade deste usuário.}
        {--group= : O grupo sob o qual o PHP é executado. Todos os arquivos serão de propriedade deste grupo.}
        {--url= : O arquivo específico para download.}
        {--release= : Uma versão específica para download do GitHub. Deixe em branco para usar o mais recente.}
        {--skip-download : Se definido, nenhum arquivo será baixado.}';

    protected $description = 'Baixa um novo arquivo do GitHub da Jexactyl PT BR e executa os comandos de atualização.';

    /**
     * Executes an upgrade command which will run through all of our standard
     * commands for Jexactyl and enable users to basically just download
     * the archive and execute this and be done.
     *
     * This places the application in maintenance mode as well while the commands
     * are being executed.
     *
     * @throws \Exception
     */
    public function handle()
    {
        $skipDownload = $this->option('skip-download');
        if (!$skipDownload) {
            $this->output->warning('Este comando não verifica a integridade dos ativos baixados. Certifique-se de que confia na fonte de download antes de continuar. Se você não deseja baixar um arquivo, indique isso usando o sinalizador --skip-download ou respondendo "no" à pergunta abaixo.');
            $this->output->comment('Fonte de download (configurado com --url=):');
            $this->line($this->getUrl());
        }

        if (version_compare(PHP_VERSION, '8.0') < 0) {
            $this->error('Não é possível executar o processo de atualização automática. A versão mínima exigida do PHP é 8.0, você tem [' . PHP_VERSION . '].');
        }

        $user = 'www-data';
        $group = 'www-data';
        if ($this->input->isInteractive()) {
            if (!$skipDownload) {
                $skipDownload = !$this->confirm('Você gostaria de baixar e descompactar os arquivos compactados para a versão mais recente?', true);
            }

            if (is_null($this->option('user'))) {
                $userDetails = posix_getpwuid(fileowner('public'));
                $user = $userDetails['name'] ?? 'www-data';

                if (!$this->confirm("O usuário do seu servidor web foi detectado como <fg=blue>[{$user}]:</> está correto?", true)) {
                    $user = $this->anticipate(
                        'Insira o nome do usuário que está executando o processo do servidor da web. Isso varia de sistema para sistema, mas geralmente é "www-data", "nginx" ou "apache".',
                        [
                            'www-data',
                            'nginx',
                            'apache',
                        ]
                    );
                }
            }

            if (is_null($this->option('group'))) {
                $groupDetails = posix_getgrgid(filegroup('public'));
                $group = $groupDetails['name'] ?? 'www-data';

                if (!$this->confirm("Seu grupo de servidores da web foi detectado como <fg=blue>[{$group}]:</> está correto?", true)) {
                    $group = $this->anticipate(
                        'Insira o nome do grupo que está executando o processo do seu servidor da web. Normalmente, este é o mesmo que o seu usuário.',
                        [
                            'www-data',
                            'nginx',
                            'apache',
                        ]
                    );
                }
            }

            if (!$this->confirm('Tem certeza que deseja executar o processo de atualização do seu Painel?')) {
                $this->warn('Processo de atualização encerrado pelo usuário.');

                return;
            }
        }

        ini_set('output_buffering', '0');
        $bar = $this->output->createProgressBar($skipDownload ? 9 : 10);
        $bar->start();

        if (!$skipDownload) {
            $this->withProgress($bar, function () {
                $this->line("\$upgrader> curl -L \"{$this->getUrl()}\" | tar -xzv");
                $process = Process::fromShellCommandline("curl -L \"{$this->getUrl()}\" | tar -xzv");
                $process->run(function ($type, $buffer) {
                    $this->{$type === Process::ERR ? 'error' : 'line'}($buffer);
                });
            });
        }

        $this->withProgress($bar, function () {
            $this->line('$upgrader> php artisan down');
            $this->call('down');
        });

        $this->withProgress($bar, function () {
            $this->line('$upgrader> chmod -R 755 storage bootstrap/cache');
            $process = new Process(['chmod', '-R', '755', 'storage', 'bootstrap/cache']);
            $process->run(function ($type, $buffer) {
                $this->{$type === Process::ERR ? 'error' : 'line'}($buffer);
            });
        });

        $this->withProgress($bar, function () {
            $command = ['composer', 'install', '--no-ansi'];
            if (config('app.env') === 'production' && !config('app.debug')) {
                $command[] = '--optimize-autoloader';
                $command[] = '--no-dev';
            }

            $this->line('$upgrader> ' . implode(' ', $command));
            $process = new Process($command);
            $process->setTimeout(10 * 60);
            $process->run(function ($type, $buffer) {
                $this->line($buffer);
            });
        });

        /** @var \Illuminate\Foundation\Application $app */
        $app = require __DIR__ . '/../../../bootstrap/app.php';
        /** @var \Pterodactyl\Console\Kernel $kernel */
        $kernel = $app->make(Kernel::class);
        $kernel->bootstrap();
        $this->setLaravel($app);

        $this->withProgress($bar, function () {
            $this->line('$upgrader> php artisan view:clear');
            $this->call('view:clear');
        });

        $this->withProgress($bar, function () {
            $this->line('$upgrader> php artisan config:clear');
            $this->call('config:clear');
        });

        $this->withProgress($bar, function () {
            $this->line('$upgrader> php artisan migrate --force --seed');
            $this->call('migrate', ['--force' => true, '--seed' => true]);
        });

        $this->withProgress($bar, function () use ($user, $group) {
            $this->line("\$upgrader> chown -R {$user}:{$group} *");
            $process = Process::fromShellCommandline("chown -R {$user}:{$group} *", $this->getLaravel()->basePath());
            $process->setTimeout(10 * 60);
            $process->run(function ($type, $buffer) {
                $this->{$type === Process::ERR ? 'error' : 'line'}($buffer);
            });
        });

        $this->withProgress($bar, function () {
            $this->line('$upgrader> php artisan queue:restart');
            $this->call('queue:restart');
        });

        $this->withProgress($bar, function () {
            $this->line('$upgrader> php artisan up');
            $this->call('up');
        });

        $this->newLine(2);
        $this->info('O painel foi atualizado com sucesso. Certifique-se de atualizar também todas as instâncias da Wings: https://pterodactyl.io/wings/1.0/upgrading.html');
    }

    protected function withProgress(ProgressBar $bar, \Closure $callback)
    {
        $bar->clear();
        $callback();
        $bar->advance();
        $bar->display();
    }

    protected function getUrl(): string
    {
        if ($this->option('url')) {
            return $this->option('url');
        }

        return sprintf(self::DEFAULT_URL, $this->option('release') ? 'download/v' . $this->option('release') : 'latest/download');
    }
}
