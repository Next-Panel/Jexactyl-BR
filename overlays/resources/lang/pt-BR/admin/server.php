<?php

// Tradução pt-BR das mensagens administrativas de servidores.
// Node, Nest, Egg, daemon, build e Docker mantêm a terminologia técnica do Jexactyl.

return [
    'exceptions' => [
        'no_new_default_allocation' => 'Você está tentando excluir a alocação padrão deste servidor, mas não há uma alocação de reserva para usar.',
        'marked_as_failed' => 'Este servidor foi marcado como tendo falhado em uma instalação anterior. O status atual não pode ser alternado neste estado.',
        'bad_variable' => 'Ocorreu um erro de validação com a variável :name.',
        'daemon_exception' => 'Ocorreu uma exceção ao tentar se comunicar com o daemon, resultando em um código de resposta HTTP/:code. Esta exceção foi registrada. (ID da solicitação: :request_id)',
        'default_allocation_not_found' => 'A alocação padrão solicitada não foi encontrada nas alocações deste servidor.',
    ],
    'alerts' => [
        'startup_changed' => 'A configuração de inicialização deste servidor foi atualizada. Se o Nest ou o Egg deste servidor foi alterado, uma reinstalação ocorrerá agora.',
        'server_deleted' => 'O servidor foi excluído do sistema com sucesso.',
        'server_created' => 'O servidor foi criado no Painel com sucesso. Aguarde alguns minutos para que o daemon conclua a instalação deste servidor.',
        'build_updated' => 'Os detalhes de build deste servidor foram atualizados. Algumas alterações podem exigir uma reinicialização para ter efeito.',
        'suspension_toggled' => 'O status de suspensão do servidor foi alterado para :status.',
        'rebuild_on_boot' => 'Este servidor foi marcado para reconstruir o contêiner Docker. Isso ocorrerá na próxima vez que o servidor for iniciado.',
        'install_toggled' => 'O status de instalação deste servidor foi alternado.',
        'server_reinstalled' => 'Este servidor foi colocado na fila para reinstalação, que começa agora.',
        'details_updated' => 'Os detalhes do servidor foram atualizados com sucesso.',
        'docker_image_updated' => 'A imagem Docker padrão usada por este servidor foi alterada com sucesso. É necessário reiniciar para aplicar esta alteração.',
        'node_required' => 'Você precisa ter pelo menos um Node configurado antes de adicionar um servidor a este Painel.',
        'transfer_nodes_required' => 'Você precisa ter pelo menos dois Nodes configurados antes de transferir servidores.',
        'transfer_started' => 'A transferência do servidor foi iniciada.',
        'transfer_not_viable' => 'O Node selecionado não tem espaço em disco ou memória disponível suficiente para acomodar este servidor.',
    ],
];
