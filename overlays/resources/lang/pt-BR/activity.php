<?php

/**
 * Contains all of the translation strings for different activity log
 * events. These should be keyed by the value in front of the colon (:)
 * in the event name. If there is no colon present, they should live at
 * the top level.
 */
return [
    'auth' => [
        'fail' => 'Falha ao entrar',
        'success' => 'Sessão iniciada',
        'password-reset' => 'Senha redefinida',
        'reset-password' => 'Solicitou redefinição de senha',
        'checkpoint' => 'Autenticação de dois fatores solicitada',
        'recovery-token' => 'Usou token de recuperação de dois fatores',
        'token' => 'Concluiu o desafio de dois fatores',
        'passkey' => 'Entrou com uma passkey',
        'ip-blocked' => 'Bloqueou uma solicitação de endereço IP não listado para :identifier',
        'sftp' => [
            'fail' => 'Falha ao entrar via SFTP',
        ],
    ],
    'user' => [
        'user' => [
            'create' => 'Criou um novo usuário :email',
        ],
        'account' => [
            'email-changed' => 'Alterou o endereço de e-mail de :old para :new',
            'password-changed' => 'Alterou a senha',
        ],
        'api-key' => [
            'create' => 'Criou uma nova chave de API :identifier',
            'delete' => 'Excluiu a chave de API :identifier',
        ],
        'ssh-key' => [
            'create' => 'Adicionou a chave SSH :fingerprint à conta',
            'delete' => 'Removeu a chave SSH :fingerprint da conta',
        ],
        'two-factor' => [
            'create' => 'Ativou a autenticação de dois fatores',
            'delete' => 'Desativou a autenticação de dois fatores',
        ],
        'passkey' => [
            'create' => 'Adicionou a passkey :name à conta',
            'delete' => 'Removeu a passkey :name da conta',
        ],
    ],
    'server' => [
        'reinstall' => 'Reinstalou o servidor',
        'console' => [
            'command' => 'Executou ":command" no servidor',
        ],
        'power' => [
            'start' => 'Iniciou o servidor',
            'stop' => 'Parou o servidor',
            'restart' => 'Reiniciou o servidor',
            'kill' => 'Encerrou o processo do servidor',
        ],
        'backup' => [
            'download' => 'Baixou o backup :name',
            'delete' => 'Excluiu o backup :name',
            'restore' => 'Restaurou o backup :name (arquivos excluídos: :truncate)',
            'restore-complete' => 'Concluiu a restauração do backup :name',
            'restore-failed' => 'Falhou ao concluir a restauração do backup :name',
            'start' => 'Iniciou um novo backup :name',
            'complete' => 'Marcou o backup :name como concluído',
            'fail' => 'Marcou o backup :name como malsucedido',
            'lock' => 'Bloqueou o backup :name',
            'unlock' => 'Desbloqueou o backup :name',
        ],
        'database' => [
            'create' => 'Criou um novo banco de dados :name',
            'rotate-password' => 'Redefiniu a senha do banco de dados :name',
            'delete' => 'Excluiu o banco de dados :name',
        ],
        'file' => [
            'compress_one' => 'Compactou :directory:files.0',
            'compress_other' => 'Compactou :count arquivos em :directory',
            'read' => 'Visualizou o conteúdo de :file',
            'copy' => 'Criou uma cópia de :file',
            'create-directory' => 'Criou o diretório :directory:name',
            'decompress' => 'Descompactou :files em :directory',
            'delete_one' => 'Excluiu :directory:files.0',
            'delete_other' => 'Excluiu :count arquivos em :directory',
            'download' => 'Baixou :file',
            'pull' => 'Baixou um arquivo remoto de :url para :directory',
            'rename_one' => 'Renomeou :directory:files.0.from para :directory:files.0.to',
            'rename_other' => 'Renomeou :count arquivos em :directory',
            'write' => 'Gravou novo conteúdo em :file',
            'upload' => 'Iniciou o envio de um arquivo',
            'uploaded' => 'Enviou :directory:file',
        ],
        'sftp' => [
            'denied' => 'Bloqueou o acesso via SFTP devido a permissões',
            'create_one' => 'Criou :files.0',
            'create_other' => 'Criou :count novos arquivos',
            'write_one' => 'Modificou o conteúdo de :files.0',
            'write_other' => 'Modificou o conteúdo de :count arquivos',
            'delete_one' => 'Excluiu :files.0',
            'delete_other' => 'Excluiu :count arquivos',
            'create-directory_one' => 'Criou o diretório :files.0',
            'create-directory_other' => 'Criou :count diretórios',
            'rename_one' => 'Renomeou :files.0.from para :files.0.to',
            'rename_other' => 'Renomeou ou moveu :count arquivos',
        ],
        'allocation' => [
            'create' => 'Adicionou :allocation ao servidor',
            'notes' => 'Atualizou as notas de :allocation de ":old" para ":new"',
            'primary' => 'Definiu :allocation como a alocação principal do servidor',
            'delete' => 'Excluiu a alocação :allocation',
        ],
        'schedule' => [
            'create' => 'Criou o agendamento :name',
            'update' => 'Atualizou o agendamento :name',
            'execute' => 'Executou manualmente o agendamento :name',
            'delete' => 'Excluiu o agendamento :name',
        ],
        'task' => [
            'create' => 'Criou uma nova tarefa ":action" para o agendamento :name',
            'update' => 'Atualizou a tarefa ":action" do agendamento :name',
            'delete' => 'Excluiu uma tarefa do agendamento :name',
        ],
        'settings' => [
            'rename' => 'Renomeou o servidor de :old para :new',
            'description' => 'Alterou a descrição do servidor de :old para :new',
        ],
        'startup' => [
            'edit' => 'Alterou a variável :variable de ":old" para ":new"',
            'image' => 'Atualizou a imagem Docker do servidor de :old para :new',
        ],
        'subuser' => [
            'create' => 'Adicionou :email como subusuário',
            'update' => 'Atualizou as permissões do subusuário :email',
            'delete' => 'Removeu :email como subusuário',
        ],
    ],
    'admin' => [
        'ai' => [
            'update' => 'Configurações do JexpanelAI atualizadas',
        ],
        'alert' => [
            'update' => 'Sistema de alertas atualizado com novos dados',
        ],
        'api-keys' => [
            'create' => 'Nova chave de API da aplicação criada',
            'delete' => 'Chave de API da aplicação excluída',
        ],
        'auth' => [
            'module' => [
                'disable' => 'Módulo de autenticação desativado',
                'enable' => 'Módulo de autenticação ativado',
                'update' => 'Configuração de módulo de autenticação atualizada',
            ],
        ],
        'billing' => [
            'categories' => [
                'create' => 'Categoria de faturamento criada',
                'delete' => 'Categoria de faturamento excluída',
                'update' => 'Categoria de faturamento atualizada',
            ],
            'config' => [
                'export' => 'Configuração de faturamento do Jexpanel exportada',
                'import' => 'Configuração de faturamento do Jexpanel importada',
            ],
            'discount-codes' => [
                'create' => 'Cupom de desconto criado',
                'delete' => 'Cupom de desconto excluído',
                'update' => 'Cupom de desconto atualizado',
            ],
            'exception-resolve' => 'Exceção de faturamento resolvida',
            'exception-resolve-all' => 'Todas as exceções de faturamento foram resolvidas',
            'products' => [
                'create' => 'Novo produto de faturamento criado',
                'delete' => 'Produto de faturamento excluído',
                'update' => 'Produto de faturamento atualizado',
            ],
            'reset-keys' => 'Credenciais de faturamento redefinidas',
            'update' => 'Configurações de faturamento do Jexpanel atualizadas',
        ],
        'database-hosts' => [
            'create' => 'Novo host de banco de dados criado',
            'delete' => 'Host de banco de dados excluído',
            'update' => 'Host de banco de dados atualizado',
        ],
        'eggs' => [
            'create' => 'Egg criado',
            'delete' => 'Egg excluído',
            'export' => 'Egg exportado',
            'update' => 'Egg atualizado',
            'variables' => [
                'create' => 'Variável adicionada a um Egg',
                'delete' => 'Variável removida de um Egg',
                'update' => 'Variáveis do Egg atualizadas',
            ],
        ],
        'link' => [
            'create' => 'Novo link personalizado da interface criado',
            'delete' => 'Link personalizado existente excluído',
            'update' => 'Link personalizado existente atualizado',
        ],
        'nests' => [
            'create' => 'Nest criado',
            'delete' => 'Nest excluído',
            'import' => 'Egg importado para um Nest',
            'update' => 'Nest atualizado',
        ],
        'nodes' => [
            'allocations' => [
                'create' => 'Alocações adicionadas a um Node',
                'delete' => 'Alocação removida de um Node',
                'delete-all' => 'Todas as alocações não utilizadas removidas de um Node',
            ],
            'create' => 'Node criado',
            'delete' => 'Node excluído',
            'update' => 'Node atualizado',
        ],
        'roles' => [
            'create' => 'Função de administrador criada',
            'delete' => 'Função de administrador excluída',
            'update' => 'Função de administrador atualizada',
            'update-permissions' => 'Permissões atualizadas para uma função de administrador',
        ],
        'server_presets' => [
            'create' => 'Preset de servidor criado',
            'delete' => 'Preset de servidor excluído',
            'update' => 'Preset de servidor atualizado',
        ],
        'servers' => [
            'bulk-power' => 'Ação de energia em massa executada em vários servidores',
            'create' => 'Servidor criado via preset de servidor',
            'databases' => [
                'create' => 'Banco de dados criado para um servidor',
                'delete' => 'Banco de dados do servidor excluído',
                'reset-password' => 'Senha do banco de dados do servidor redefinida',
            ],
            'delete' => 'Servidor excluído',
            'reinstall' => 'Servidor marcado para reinstalação',
            'startup' => 'Configuração de inicialização do servidor atualizada',
            'suspend' => 'Servidor suspenso',
            'toggle' => 'Estado de instalação do servidor alternado',
            'transfer' => 'Transferência de servidor iniciada',
            'unsuspend' => 'Servidor reativado',
            'update' => 'Servidor atualizado',
        ],
        'settings' => [
            'mode' => 'O modo do painel foi alterado',
            'update' => 'As configurações gerais do painel foram atualizadas',
        ],
        'setup' => [
            'finish' => 'O assistente de configuração do painel foi concluído',
        ],
        'theme' => [
            'reset' => 'O tema do painel foi redefinido para o padrão',
            'update' => 'Uma cor do tema do painel foi atualizada',
        ],
        'tickets' => [
            'create' => 'Ticket criado',
            'delete' => 'Ticket excluído',
            'message' => 'Mensagem adicionada a um ticket',
            'settings' => 'Configurações do módulo de tickets atualizadas',
            'update' => 'Ticket atualizado',
        ],
        'users' => [
            'create' => 'Usuário criado',
            'delete' => 'Usuário excluído',
            'suspend' => 'Usuário suspenso',
            'update' => 'Usuário atualizado',
        ],
        'webhooks' => [
            'test' => 'A integração de webhook foi testada',
            'toggle' => 'Evento(s) de webhook alternado(s)',
            'update' => 'Configurações de webhook do Jexpanel atualizadas',
        ],
    ],
];
