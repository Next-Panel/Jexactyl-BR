<?php
/**
 * Pterodactyl - Panel
 * Copyright (c) 2015 - 2017 Dane Everitt <dane@daneeveritt.com>.
 *
 * This software is licensed under the terms of the MIT license.
 * https://opensource.org/licenses/MIT
 */

return [
    'validation' => [
        'fqdn_not_resolvable' => 'O endereço FQDN ou IP fornecido não resolve um endereço IP válido.',
        'fqdn_required_for_ssl' => 'Um nome de domínio totalmente qualificado que se resolve com um endereço IP público é necessário para usar o SSL para este nó.',
    ],
    'notices' => [
        'allocations_added' => 'As alocações foram adicionadas com sucesso a este nó.',
        'node_deleted' => 'O nó foi removido com sucesso do painel.',
        'location_required' => 'Você deve ter pelo menos um local configurado antes de poder adicionar um nó a este painel.',
        'node_created' => 'Criou com sucesso um novo nó. Você pode configurar automaticamente o daemon nesta máquina visitando o \'Configuration\' tab. <strong>Antes de adicionar quaisquer servidores, você deve primeiro alocar pelo menos um endereço IP e porta.</strong>',
        'node_updated' => 'As informações do nó foram atualizadas. Se alguma configuração de daemon foi alterada, você precisará reiniciá-la para que essas alterações entrem em vigor.',
        'unallocated_deleted' => 'Excluiu todas as portas não alocadas para <code>:ip</code>.',
    ],
];
