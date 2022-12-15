<?php

namespace Pterodactyl\Console\Commands\Node;

use Pterodactyl\Models\Node;
use Illuminate\Console\Command;

class NodeConfigurationCommand extends Command
{
    protected $signature = 'p:node:configuration
                            {node : O ID ou UUID do node para retornar a configuração.}
                            {--format=yaml : O formato de saída. As opções são "yaml" e "json".}';

    protected $description = 'Exibe a configuração para o node especificado.';

    public function handle(): int
    {
        $column = ctype_digit((string) $this->argument('node')) ? 'id' : 'uuid';

        /** @var \Pterodactyl\Models\Node $node */
        $node = Node::query()->where($column, $this->argument('node'))->firstOr(function () {
            $this->error('O node selecionado não existe.');

            exit(1);
        });

        $format = $this->option('format');
        if (!in_array($format, ['yaml', 'yml', 'json'])) {
            $this->error('Formato inválido especificado. As opções válidas são "yaml" e "json".');

            return 1;
        }

        if ($format === 'json') {
            $this->output->write($node->getJsonConfiguration(true));
        } else {
            $this->output->write($node->getYamlConfiguration());
        }

        $this->output->newLine();

        return 0;
    }
}
