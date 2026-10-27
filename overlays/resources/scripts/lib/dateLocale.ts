import { ptBR } from 'date-fns/locale';
import type { Locale } from 'date-fns';

// Locale pt-BR fixo para as datas da interface. A UI deste painel e
// integralmente traduzida para pt-BR e nenhum outro texto visivel leva
// em conta a coluna legada users.language; seguir o <html lang> aqui
// misturaria datas em ingles no meio de uma interface em portugues
// (era o caso de contas criadas com o padrao 'en' do banco).
export function dateLocale(): Locale {
    return ptBR;
}
