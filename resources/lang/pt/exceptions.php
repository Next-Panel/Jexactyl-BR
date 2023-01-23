<?php

return [
    'daemon_connection_failed' => 'Houve uma exceção ao tentar se comunicar com o daemon resultando em um código de resposta http/:code. Esta exceção foi registrada.',
    'node' => [
        'servers_attached' => 'Um Node não deve ter servidores vinculados a ele para ser excluído.',
        'daemon_off_config_updated' => 'A configuração daemon <strong> foi atualizada </strong>, no entanto, houve um erro encontrado ao tentar atualizar automaticamente o arquivo de configuração no Daemon. Você precisará atualizar manualmente o arquivo de configuração (config.yml) para que o daemon aplique essas alterações.',
    ],
    'allocations' => [
        'server_using' => 'Um servidor é atualmente atribuído a essa alocação. Uma alocação só pode ser excluída se nenhum servidor for atribuído no momento.',
        'too_many_ports' => 'Adicionar mais de 1000 portas em um único intervalo ao mesmo tempo não é suportado.',
        'invalid_mapping' => 'O mapeamento previsto para :port era inválido e não pôde ser processado.',
        'cidr_out_of_range' => 'CIDR a notação só permite máscaras entre /25 e /32.',
        'port_out_of_range' => 'Os portas em uma alocação devem ser maiores que 1024 e menores ou iguais a 65535.',
    ],
    'nest' => [
        'delete_has_servers' => 'Um Nest com servidores ativos conectados a ele não pode ser excluído do Painel.',
        'egg' => [
            'delete_has_servers' => 'Um Egg com servidores ativos conectados a ele não pode ser excluído do Painel.',
            'invalid_copy_id' => 'O Egg selecionado para copiar um script de qualquer um não existe ou está copiando um script em si.',
            'must_be_child' => 'A diretiva "Copiar configurações a partir" para este Egg deve ser uma opção Child(Propriada)  para o Nest selecionado.',
            'has_children' => 'Este Egg é pai de um ou mais Eggs. Por favor, exclua esses Eggs antes de excluir este Egg.',
        ],
        'variables' => [
            'env_not_unique' => 'A variável ambiente :name deve ser único para este Egg.',
            'reserved_name' => 'A variável ambiente :name é protegido e não pode ser atribuído a uma variável.',
            'bad_validation_rule' => 'A regra de validação ":rule" não é uma regra válida para este aplicativo.',
        ],
        'importer' => [
            'json_error' => 'Houve um erro ao tentar analisar o arquivo JSON: :error.',
            'file_error' => 'O arquivo JSON fornecido não era válido.',
            'invalid_json_provided' => 'O arquivo JSON fornecido não está em um formato que possa ser reconhecido.',
        ],
    ],
    'subusers' => [
        'editing_self' => 'A edição de sua própria conta subusuário não é permitida.',
        'user_is_owner' => 'Você não pode adicionar o proprietário do servidor como subusário para este servidor.',
        'subuser_exists' => 'Um usuário com esse endereço de e-mail já está atribuído como subusuário para este servidor.',
    ],
    'databases' => [
        'delete_has_databases' => 'Não é possível excluir um servidor host de banco de dados que tenha bancos de dados ativos vinculados a ele.',
    ],
    'tasks' => [
        'chain_interval_too_long' => 'O tempo máximo de intervalo para uma tarefa acorrentada é de 15 minutos.',
    ],
    'locations' => [
        'has_nodes' => 'Não é possível excluir uma localização que tenha Nodes ativos e anexados a ela.',
    ],
    'users' => [
        'node_revocation_failed' => 'Falhou em revogar chaves em <a href=":link">Node #:node</a>. :error',
    ],
    'deployment' => [
        'no_viable_nodes' => 'Não foi possível encontrar Nodes que atendessem os requisitos especificados para a implantação automática.',
        'no_viable_allocations' => 'Não foram encontradas alocações que satisfaçam os requisitos para implantação automática.',
    ],
    'api' => [
        'resource_not_found' => 'O recurso solicitado não existe neste servidor.',
    ],
];
