<?php

return [
    'daemon_connection_failed' => 'Ocorreu uma exceção ao tentar se comunicar com o daemon, resultando em um código de resposta HTTP/:code. Esta exceção foi registrada no log.',
    'node' => [
        'servers_attached' => 'Para excluir um Node, ele não pode ter servidores vinculados.',
        'daemon_off_config_updated' => 'A configuração do daemon foi atualizada, mas ocorreu um erro ao tentar atualizar automaticamente o arquivo de configuração no daemon. Você precisará atualizar manualmente o arquivo de configuração (config.yml) para que o daemon aplique essas alterações.',
    ],
    'allocations' => [
        'server_using' => 'Um servidor está atribuído a esta alocação. Uma alocação só pode ser excluída se nenhum servidor estiver atribuído a ela.',
        'too_many_ports' => 'Não é possível adicionar mais de 1000 portas em um único intervalo de uma vez.',
        'invalid_mapping' => 'O mapeamento informado para :port é inválido e não pôde ser processado.',
        'cidr_out_of_range' => 'A notação CIDR só permite máscaras entre /25 e /32.',
        'port_out_of_range' => 'As portas em uma alocação devem ser maiores que 1024 e menores ou iguais a 65535.',
    ],
    'nest' => [
        'delete_has_servers' => 'Um Nest com servidores ativos vinculados não pode ser excluído do Painel.',
        'egg' => [
            'delete_has_servers' => 'Um Egg com servidores ativos vinculados não pode ser excluído do Painel.',
            'invalid_copy_id' => 'O Egg selecionado para copiar um script não existe ou já é uma cópia de outro Egg.',
            'must_be_child' => 'A diretiva "Copiar configurações de" deste Egg deve ser uma opção filha do Nest selecionado.',
            'has_children' => 'Este Egg é pai de um ou mais outros Eggs. Exclua esses Eggs antes de excluir este Egg.',
        ],
        'variables' => [
            'env_not_unique' => 'A variável de ambiente :name deve ser exclusiva deste Egg.',
            'reserved_name' => 'A variável de ambiente :name é protegida e não pode ser usada como nome de uma variável.',
            'bad_validation_rule' => 'A regra de validação ":rule" não é uma regra válida para esta aplicação.',
        ],
        'importer' => [
            'json_error' => 'Ocorreu um erro ao tentar analisar o arquivo JSON: :error.',
            'file_error' => 'O arquivo JSON fornecido não é válido.',
            'invalid_json_provided' => 'O arquivo JSON fornecido não está em um formato reconhecível.',
        ],
    ],
    'subusers' => [
        'editing_self' => 'Não é permitido editar sua própria conta de subusuário.',
        'user_is_owner' => 'Você não pode adicionar o proprietário do servidor como subusuário deste servidor.',
        'subuser_exists' => 'Um usuário com esse endereço de e-mail já está atribuído como subusuário deste servidor.',
    ],
    'databases' => [
        'delete_has_databases' => 'Não é possível excluir um servidor host de banco de dados que tenha bancos de dados ativos vinculados.',
    ],
    'tasks' => [
        'chain_interval_too_long' => 'O intervalo máximo para uma tarefa encadeada é de 15 minutos.',
    ],
    'users' => [
        'node_revocation_failed' => 'Falha ao revogar as chaves no <a href=":link">Node #:node</a>. :error',
    ],
    'deployment' => [
        'no_viable_nodes' => 'Nenhum Node que atenda aos requisitos especificados para implantação automática foi encontrado.',
        'no_viable_allocations' => 'Nenhuma alocação que atenda aos requisitos para implantação automática foi encontrada.',
    ],
    'api' => [
        'resource_not_found' => 'O recurso solicitado não existe neste servidor.',
    ],
    'billing' => [
        'no_visible_categories_title' => 'Nenhuma categoria de produto visível',
        'no_visible_categories_description' => 'Crie uma categoria e marque-a como visível',
        'no_products_in_category_title' => 'Nenhum produto na categoria :category está visível',
        'no_products_in_category_description' => 'Acesse essa categoria e crie um produto visível',
        'no_deployable_nodes_title' => 'Nenhum Node implantável encontrado',
        'no_deployable_nodes_description' => 'Marque a caixa "implantável" em ao menos um Node',
        'no_nodes_satisfy_requirements_title' => 'Nenhum Node atende aos requisitos',
        'no_nodes_satisfy_requirements_description' => 'Os Nodes disponíveis estão offline ou sem alocações livres',
        'failed_create_billable_server_title' => 'Falha ao criar o servidor pago',
        'failed_find_allocation_title' => 'Falha ao encontrar alocação para o servidor',
        'failed_find_allocation_description' => 'Crie mais alocações (portas) para o Node :node',
        'failed_create_free_server_title' => 'Falha ao criar o servidor gratuito',
        'failed_generate_invoice_title' => 'Falha ao gerar a fatura',
        'deployment_renewal_failed_title' => 'Falha na implantação ou renovação do servidor',
    ],
];
