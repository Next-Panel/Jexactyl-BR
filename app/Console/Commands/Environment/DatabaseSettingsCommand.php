<?php

namespace Pterodactyl\Console\Commands\Environment;

use PDOException;
use Illuminate\Console\Command;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Database\DatabaseManager;
use Pterodactyl\Traits\Commands\EnvironmentWriterTrait;

class DatabaseSettingsCommand extends Command
{
    use EnvironmentWriterTrait;

    protected $description = 'Defina as configurações do database para o painel.';

    protected $signature = 'p:environment:database
                            {--host= : O endereço de conexão para o servidor MySQL.}
                            {--port= : A porta de conexão para o servidor MySQL.}
                            {--database= : O database a ser usado.}
                            {--username= : Nome de usuário para usar ao conectar.}
                            {--password= : Senha a ser usada para este database.}';

    protected array $variables = [];

    /**
     * DatabaseSettingsCommand constructor.
     */
    public function __construct(private DatabaseManager $database, private Kernel $console)
    {
        parent::__construct();
    }

    /**
     * Handle command execution.
     *
     * @throws \Pterodactyl\Exceptions\PterodactylException
     */
    public function handle(): int
    {
        $this->output->note(' É altamente recomendável não usar "localhost" como seu host de database, pois temos visto problemas frequentes de conexão de soquete. Se você quiser usar uma conexão local, você deve estar usando "127.0.0.1".');
        $this->variables['DB_HOST'] = $this->option('host') ?? $this->ask(
            'Host do Database ',
            config('database.connections.mysql.host', '127.0.0.1')
        );

        $this->variables['DB_PORT'] = $this->option('port') ?? $this->ask(
            'Porta do Database ',
            config('database.connections.mysql.port', 3306)
        );

        $this->variables['DB_DATABASE'] = $this->option('database') ?? $this->ask(
            'Nome do Database ',
            config('database.connections.mysql.database', 'panel')
        );

        $this->output->note('Usar a conta "root" para conexões MySQL não é apenas altamente desaprovado, mas também não é permitido por este aplicativo. Você precisa ter criado um usuário MySQL para este software.');
        $this->variables['DB_USERNAME'] = $this->option('username') ?? $this->ask(
            'Usuário do Database',
            config('database.connections.mysql.username', 'pterodactyl')
        );

        $askForMySQLPassword = true;
        if (!empty(config('database.connections.mysql.password')) && $this->input->isInteractive()) {
            $this->variables['DB_PASSWORD'] = config('database.connections.mysql.password');
            $askForMySQLPassword = $this->confirm('Parece que você já tem uma senha de conexão MySQL definida, você gostaria de alterá-la?');
        }

        if ($askForMySQLPassword) {
            $this->variables['DB_PASSWORD'] = $this->option('password') ?? $this->secret(' Senha do Database');
        }

        try {
            $this->testMySQLConnection();
        } catch (PDOException $exception) {
            $this->output->error(sprintf('Não é possível conectar-se ao servidor MySQL usando as credenciais fornecidas. O erro retornado foi "%s".', $exception->getMessage()));
            $this->output->error('Suas credenciais de conexão NÃO foram salvas. Você precisará fornecer informações de conexão válidas antes de prosseguir.');

            if ($this->confirm('Voltar e tentar novamente?')) {
                $this->database->disconnect('_pterodactyl_command_test');

                return $this->handle();
            }

            return 1;
        }

        $this->writeToEnvironment($this->variables);

        $this->info($this->console->output());

        return 0;
    }

    /**
     * Test that we can connect to the provided MySQL instance and perform a selection.
     */
    private function testMySQLConnection()
    {
        config()->set('database.connections._pterodactyl_command_test', [
            'driver' => 'mysql',
            'host' => $this->variables['DB_HOST'],
            'port' => $this->variables['DB_PORT'],
            'database' => $this->variables['DB_DATABASE'],
            'username' => $this->variables['DB_USERNAME'],
            'password' => $this->variables['DB_PASSWORD'],
            'charset' => 'utf8mb4',
            'collation' => 'utf8mb4_unicode_ci',
            'strict' => true,
        ]);

        $this->database->connection('_pterodactyl_command_test')->getPdo();
    }
}
