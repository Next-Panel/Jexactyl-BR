<?php

// Tradução pt-BR das mensagens administrativas de Nodes.
// Node, daemon, FQDN e Docker mantêm a terminologia técnica do Jexactyl.

return [
    'validation' => [
        'fqdn_not_resolvable' => 'O FQDN ou endereço IP informado não resolve para um endereço IP válido.',
        'fqdn_required_for_ssl' => 'É necessário um nome de domínio totalmente qualificado que resolva para um endereço IP público para usar SSL neste Node.',
    ],
    'notices' => [
        'allocations_added' => 'As alocações foram adicionadas a este Node com sucesso.',
        'node_deleted' => 'O Node foi removido do Painel com sucesso.',
        'node_created' => 'Novo Node criado com sucesso. Você pode configurar o daemon automaticamente nesta máquina acessando a aba \'Configuração\'. <strong>Antes de adicionar qualquer servidor, você precisa primeiro alocar pelo menos um endereço IP e uma porta.</strong>',
        'node_updated' => 'As informações do Node foram atualizadas. Se alguma configuração do daemon foi alterada, você precisará reiniciá-lo para que as mudanças tenham efeito.',
        'unallocated_deleted' => 'Todas as portas não alocadas de <code>:ip</code> foram excluídas.',
    ],
];
