<?php

return [
    'user' => [
        'search_users' => 'Informe um nome de usuário, ID do usuário ou endereço de e-mail',
        'select_search_user' => 'ID do usuário a excluir (digite \'0\' para buscar novamente)',
        'deleted' => 'Usuário excluído com sucesso do Painel.',
        'confirm_delete' => 'Tem certeza de que deseja excluir este usuário do Painel?',
        'no_users_found' => 'Nenhum usuário foi encontrado para o termo de busca informado.',
        'multiple_found' => 'Várias contas foram encontradas para o usuário informado; não foi possível excluir um usuário devido à flag --no-interaction.',
        'ask_admin' => 'Este usuário é administrador?',
        'ask_email' => 'Endereço de e-mail',
        'ask_username' => 'Nome de usuário',
        'ask_name_first' => 'Nome',
        'ask_name_last' => 'Sobrenome',
        'ask_password' => 'Senha',
        'ask_password_tip' => 'Se você quiser criar uma conta com uma senha aleatória enviada por e-mail ao usuário, execute este comando novamente (CTRL+C) e use a flag `--no-password`.',
        'ask_password_help' => 'As senhas devem ter pelo menos 8 caracteres e conter ao menos uma letra maiúscula e um número.',
        '2fa_help_text' => [
            'Este comando desativará a autenticação de dois fatores da conta de um usuário, caso esteja ativada. Use-o apenas como comando de recuperação de conta se o usuário estiver bloqueado da própria conta.',
            'Se não era isso que você queria fazer, pressione CTRL+C para sair deste processo.',
        ],
        '2fa_disabled' => 'A autenticação de dois fatores foi desativada para :email.',
    ],
    'schedule' => [
        'output_line' => 'Enviando o job da primeira tarefa em `:schedule` (:hash).',
    ],
    'maintenance' => [
        'deleting_service_backup' => 'Excluindo o arquivo de backup do serviço :file.',
    ],
    'server' => [
        'rebuild_failed' => 'A solicitação de reconstrução de ":name" (#:id) no Node ":node" falhou com o erro: :message',
        'reinstall' => [
            'failed' => 'A solicitação de reinstalação de ":name" (#:id) no Node ":node" falhou com o erro: :message',
            'confirm' => 'Você está prestes a reinstalar um grupo de servidores. Deseja continuar?',
        ],
        'power' => [
            'confirm' => 'Você está prestes a executar a ação :action em :count servidores. Deseja continuar?',
            'action_failed' => 'A solicitação de ação de energia de ":name" (#:id) no Node ":node" falhou com o erro: :message',
        ],
    ],
];
