<?php

return [
    'permissions' => [
        'websocket_*' => 'Permite acesso ao websocket para este servidor.',
        'control_console' => 'Permite que o usuário envie dados para o console do servidor.',
        'control_start' => 'Permite que o usuário inicie a instância do servidor.',
        'control_stop' => 'Permite que o usuário pare a instância do servidor.',
        'control_restart' => 'Permite que o usuário reinicie a instância do servidor.',
        'control_kill' => 'Permite que o usuário mate a instância do servidor.',
        'user_create' => 'Permite que o usuário crie novas contas de usuário para o servidor.',
        'user_read' => 'Permite que o usuário permissão para visualizar usuários associados a este servidor.',
        'user_update' => 'Permite que o usuário modifique outros usuários associados a este servidor.',
        'user_delete' => 'Permite que o usuário exclua outros usuários associados a este servidor.',
        'file_create' => 'Permite que o usuário crie novas contas de usuário para o servidor.',
        'file_read' => 'Permite que o usuário veja arquivos e pastas associados a esta instância do servidor, bem como visualize seu conteúdo.',
        'file_update' => 'Permite que o usuário atualize arquivos e pastas associadas ao servidor.',
        'file_delete' => 'Permite que o usuário exclua arquivos e diretórios.',
        'file_archive' => 'Permite que o usuário crie arquivos de arquivos e descomprima os arquivos existentes.',
        'file_sftp' => 'Permite que o usuário execute as ações de arquivo acima usando um cliente SFTP.',
        'allocation_read' => 'Permite o acesso às páginas de gerenciamento de alocação de servidores.',
        'allocation_update' => 'Permite que o usuário faça modificações no server\'s Alocações.',
        'database_create' => 'Permite que o usuário crie um novo banco de dados para o servidor.',
        'database_read' => 'Permite que o usuário visualize os bancos de dados do servidor.',
        'database_update' => 'Permite que um usuário permissão para fazer modificações em um banco de dados. Se o usuário não tiver a permissão "Exibir senha", também não poderá modificar a senha.',
        'database_delete' => 'Permite que um usuário exclua uma instância de banco de dados.',
        'database_view_password' => 'Permite que um usuário exa coma a senha do banco de dados no sistema.',
        'schedule_create' => 'Permite que um usuário crie um novo cronograma para o servidor.',
        'schedule_read' => 'Permite que um usuário permissão para visualizar horários para um servidor.',
        'schedule_update' => 'Permite que um usuário permissão para fazer modificações em um cronograma de servidor existente.',
        'schedule_delete' => 'Permite que um usuário exclua um cronograma para o servidor.',
    ],
];
