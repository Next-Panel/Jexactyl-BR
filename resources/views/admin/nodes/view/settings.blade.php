@extends('layouts.admin')

@section('title')
    {{ $node->name }}: Definições
@endsection

@section('content-header')
    <h1>{{ $node->name }}<small>Defina as definições do node.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li><a href="{{ route('admin.nodes') }}">Nodes</a></li>
        <li><a href="{{ route('admin.nodes.view', $node->id) }}">{{ $node->name }}</a></li>
        <li class="active">Definições</li>
    </ol>
@endsection

@section('content')
<div class="row">
    <div class="col-xs-12">
        <div class="nav-tabs-custom nav-tabs-floating">
            <ul class="nav nav-tabs">
                <li><a href="{{ route('admin.nodes.view', $node->id) }}">Sobre</a></li>
                <li class="active"><a href="{{ route('admin.nodes.view.settings', $node->id) }}">Definições</a></li>
                <li><a href="{{ route('admin.nodes.view.configuration', $node->id) }}">Configuração</a></li>
                <li><a href="{{ route('admin.nodes.view.allocation', $node->id) }}">Alocações</a></li>
                <li><a href="{{ route('admin.nodes.view.servers', $node->id) }}">Servidores</a></li>
            </ul>
        </div>
    </div>
</div>
<form action="{{ route('admin.nodes.view.settings', $node->id) }}" method="POST">
    <div class="row">
        <div class="col-sm-6">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Definições</h3>
                </div>
                <div class="box-body row">
                    <div class="form-group col-xs-12">
                        <label for="name" class="control-label">Nome do Node</label>
                        <div>
                            <input type="text" autocomplete="off" name="name" class="form-control" value="{{ old('name', $node->name) }}" />
                            <p class="text-muted"><small>Limites de caracteres: <code>a-zA-Z0-9_.-</code> e <code>[Espaço]</code> (min 1, max 100 caracteres).</small></p>
                        </div>
                    </div>
                    <div class="form-group col-xs-12">
                        <label for="description" class="control-label">Descrição</label>
                        <div>
                            <textarea name="description" id="description" rows="4" class="form-control">{{ $node->description }}</textarea>
                        </div>
                    </div>
                    <div class="form-group col-xs-12">
                        <label for="name" class="control-label">Localização</label>
                        <div>
                            <select name="location_id" class="form-control">
                                @foreach($locations as $location)
                                    <option value="{{ $location->id }}" {{ (old('location_id', $node->location_id) === $location->id) ? 'selected' : '' }}>{{ $location->long }} ({{ $location->short }})</option>
                                @endforeach
                            </select>
                        </div>
                    </div>
                    <div class="form-group col-xs-12">
                        <label for="public" class="control-label">Permitir alocação automática <sup><a data-toggle="tooltip" data-placement="top" title="Permitir alocação automática para este Node?">?</a></sup></label>
                        <div>
                            <input type="radio" name="public" value="1" {{ (old('public', $node->public)) ? 'checked' : '' }} id="public_1" checked> <label for="public_1" style="padding-left:5px;">Sim</label><br />
                            <input type="radio" name="public" value="0" {{ (old('public', $node->public)) ? '' : 'checked' }} id="public_0"> <label for="public_0" style="padding-left:5px;">Não</label>
                        </div>
                    </div>
                    <div class="form-group col-xs-12">
                        <label for="fqdn" class="control-label">Nome de domínio totalmente qualificado</label>
                        <div>
                            <input type="text" autocomplete="off" name="fqdn" class="form-control" value="{{ old('fqdn', $node->fqdn) }}" />
                        </div>
                        <p class="text-muted"><small>Insira o nome de domínio (por exemplo,<code> node.example.com</code>) a ser usado para se conectar ao daemon. Um endereço IP só pode ser usado se você não estiver usando SSL para esse node.
                                <a tabindex="0" data-toggle="popover" data-trigger="focus" title="Por que eu preciso de uma FQDN?" data-content="A fim de proteger as comunicações entre seu servidor e este Node, usamos SSL. Não é possível gerar um certificado SSL para endereços IP, portanto, você precisará fornecer um FQDN.">Por quê?</a>
                            </small></p>
                    </div>
                    <div class="form-group col-xs-12">
                        <label for="deploy_fee" class="control-label">Taxa de implantação da loja</label>
                        <div>
                            <input type="text" autocomplete="off" name="deploy_fee" class="form-control" value="{{ old('deploy_fee', $node->deploy_fee) }}" />
                        </div>
                        <p class="text-muted"><small>Inserir um valor aqui significa que os usuários que implantam um servidor através da Storefront devem pagar uma taxa em créditos para implantar a este Node.</small></p>
                    </div>
                    <div class="form-group col-xs-12">
                        <label class="form-label"><span class="label label-warning"><i class="fa fa-power-off"></i></span> Comunique-se através de SSL</label>
                        <div>
                            <div class="radio radio-success radio-inline">
                                <input type="radio" id="pSSLTrue" value="https" name="scheme" {{ (old('scheme', $node->scheme) === 'https') ? 'checked' : '' }}>
                                <label for="pSSLTrue"> Usar conexão SSL</label>
                            </div>
                            <div class="radio radio-danger radio-inline">
                                <input type="radio" id="pSSLFalse" value="http" name="scheme" {{ (old('scheme', $node->scheme) !== 'https') ? 'checked' : '' }}>
                                <label for="pSSLFalse"> Usar conexão HTTP</label>
                            </div>
                        </div>
                        <p class="text-muted small">Na maioria dos casos, você deve optar por usar uma conexão SSL. Se estiver usando um endereço IP ou se você não deseja usar SSL, selecione uma conexão HTTP.</p>
                    </div>
                    <div class="form-group col-xs-12">
                        <label class="form-label"><span class="label label-warning"><i class="fa fa-power-off"></i></span> Serviços de CDN</label>
                        <div>
                            <div class="radio radio-success radio-inline">
                                <input type="radio" id="pProxyFalse" value="0" name="behind_proxy" {{ (old('behind_proxy', $node->behind_proxy) == false) ? 'checked' : '' }}>
                                <label for="pProxyFalse"> Não usar Proxy </label>
                            </div>
                            <div class="radio radio-info radio-inline">
                                <input type="radio" id="pProxyTrue" value="1" name="behind_proxy" {{ (old('behind_proxy', $node->behind_proxy) == true) ? 'checked' : '' }}>
                                <label for="pProxyTrue"> Usar Proxy </label>
                            </div>
                        </div>
                        <p class="text-muted small">Se você estiver usando serviços CDN como CloudFlare que estão com proxy ativados coloque <code>"Usar Proxy"</code>,Caso contrario deixe <code>"Não usar Proxy"</code>.</p>
                    </div>
                    <div class="form-group col-xs-12">
                        <label class="form-label"><span class="label label-warning"><i class="fa fa-wrench"></i></span> Modo de Manutenção</label>
                        <div>
                            <div class="radio radio-success radio-inline">
                                <input type="radio" id="pMaintenanceFalse" value="0" name="maintenance_mode" {{ (old('behind_proxy', $node->maintenance_mode) == false) ? 'checked' : '' }}>
                                <label for="pMaintenanceFalse"> Desativado</label>
                            </div>
                            <div class="radio radio-warning radio-inline">
                                <input type="radio" id="pMaintenanceTrue" value="1" name="maintenance_mode" {{ (old('behind_proxy', $node->maintenance_mode) == true) ? 'checked' : '' }}>
                                <label for="pMaintenanceTrue"> Ativado</label>
                            </div>
                        </div>
                        <p class="text-muted small">Se o Node estiver marcado como 'Em Manutenção', os usuários não poderão acessar os servidores que estão nesse node.</p>
                    </div>
                </div>
            </div>
        </div>
        <div class="col-sm-6">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Limites de alocação</h3>
                </div>
                <div class="box-body row">
                    <div class="col-xs-12">
                        <div class="row">
                            <div class="form-group col-xs-6">
                                <label for="memory" class="control-label">Memoria Total</label>
                                <div class="input-group">
                                    <input type="text" name="memory" class="form-control" data-multiplicator="true" value="{{ old('memory', $node->memory) }}"/>
                                    <span class="input-group-addon">MiB</span>
                                </div>
                            </div>
                            <div class="form-group col-xs-6">
                                <label for="memory_overallocate" class="control-label">Sobre-alocação</label>
                                <div class="input-group">
                                    <input type="text" name="memory_overallocate" class="form-control" value="{{ old('memory_overallocate', $node->memory_overallocate) }}"/>
                                    <span class="input-group-addon">%</span>
                                </div>
                            </div>
                        </div>
                        <p class="text-muted small">Insira a quantidade total de memória disponível nesse node para alocação a servidores. Você também pode fornecer uma porcentagem que pode permitir a alocação de mais do que a memória definida.</p>
                    </div>
                    <div class="col-xs-12">
                        <div class="row">
                            <div class="form-group col-xs-6">
                                <label for="disk" class="control-label">Espaço em disco</label>
                                <div class="input-group">
                                    <input type="text" name="disk" class="form-control" data-multiplicator="true" value="{{ old('disk', $node->disk) }}"/>
                                    <span class="input-group-addon">MiB</span>
                                </div>
                            </div>
                            <div class="form-group col-xs-6">
                                <label for="disk_overallocate" class="control-label">Sobre-alocação</label>
                                <div class="input-group">
                                    <input type="text" name="disk_overallocate" class="form-control" value="{{ old('disk_overallocate', $node->disk_overallocate) }}"/>
                                    <span class="input-group-addon">%</span>
                                </div>
                            </div>
                        </div>
                        <p class="text-muted small">Insira a quantidade total de espaço em disco disponível neste node para alocação de servidor. Você também pode fornecer uma porcentagem que determinará a quantidade de espaço em disco acima do limite definido a ser permitido.</p>
                    </div>
                </div>
            </div>
        </div>
        <div class="col-sm-6">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Configuração Geral</h3>
                </div>
                <div class="box-body row">
                    <div class="form-group col-xs-12">
                        <label for="disk_overallocate" class="control-label">Tamanho máximo do arquivo de upload da Web</label>
                        <div class="input-group">
                            <input type="text" name="upload_size" class="form-control" value="{{ old('upload_size', $node->upload_size) }}"/>
                            <span class="input-group-addon">MiB</span>
                        </div>
                        <p class="text-muted"><small>Insira o tamanho máximo de arquivos que podem ser carregados por meio do gerenciador de arquivos baseado na Web.</small></p>
                    </div>
                    <div class="col-xs-12">
                        <div class="row">
                            <div class="form-group col-md-6">
                                <label for="daemonListen" class="control-label"><span class="label label-warning"><i class="fa fa-power-off"></i></span> Porta do Daemon</label>
                                <div>
                                    <input type="text" name="daemonListen" class="form-control" value="{{ old('daemonListen', $node->daemonListen) }}"/>
                                </div>
                            </div>
                            <div class="form-group col-md-6">
                                <label for="daemonSFTP" class="control-label"><span class="label label-warning"><i class="fa fa-power-off"></i></span>Porta SFTP do Daemon </label>
                                <div>
                                    <input type="text" name="daemonSFTP" class="form-control" value="{{ old('daemonSFTP', $node->daemonSFTP) }}"/>
                                </div>
                            </div>
                        </div>
                        <div class="row">
                            <div class="col-md-12">
                                <p class="text-muted"><small>O daemon executa seu próprio contêiner de gerenciamento SFTP e não usa o processo SSHd no servidor físico principal. <Strong>Não use a mesma porta que você atribuiu para o processo SSH do servidor físico.</strong></small></p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div class="col-sm-6">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Implantação</h3>
                </div>
                <div class="box-body row">
                    <div class="form-group col-xs-12">
                        <label for="deployable" class="control-label">Implantável via Loja do Jexactyl</label>
                        <div>
                            <div class="radio radio-success radio-inline">
                                <input type="radio" id="pDeployableTrue" value="1" name="deployable" {{ (old('deployable', $node->deployable)) ? 'checked' : '' }}>
                                <label for="pDeployableTrue"> Permitir</label>
                            </div>
                            <div class="radio radio-danger radio-inline">
                                <input type="radio" id="pDeployableFalse" value="0" name="deployable" {{ (old('deployable', $node->deployable)) ? '' : 'checked' }}>
                                <label for="pDeployableFalse"> Negar</label>
                            </div>
                        </div>
                        <p class="text-muted"><small>
                            Essa opção permite que você controle se esse node está visível por meio da página Criação de Servidor da vitrine Jexactyl.
                            Se ele estiver definido como negado, os usuários não poderão implantar nesse node.
                        </small></p>
                    </div>
                </div>
            </div>
        </div>
        <div class="col-xs-12">
            <div class="box box-primary">
                <div class="box-header with-border">
                    <h3 class="box-title">Salvar Configurações</h3>
                </div>
                <div class="box-body row">
                    <div class="form-group col-sm-6">
                        <div>
                            <input type="checkbox" name="reset_secret" id="reset_secret" /> <label for="reset_secret" class="control-label">Redefinir chave mestra do Daemon</label>
                        </div>
                        <p class="text-muted"><small>Redefinir a chave mestra do daemon anulará qualquer solicitação proveniente da chave antiga. Essa chave é usada para todas as operações confidenciais no daemon, incluindo criação e exclusão do servidor. Sugerimos alterar essa chave regularmente por segurança.</small></p>
                    </div>
                </div>
                <div class="box-footer">
                    {!! method_field('PATCH') !!}
                    {!! csrf_field() !!}
                    <button type="submit" class="btn btn-primary pull-right">Salvar mudanças</button>
                </div>
            </div>
        </div>
    </div>
</form>
@endsection

@section('footer-scripts')
    @parent
    <script>
    $('[data-toggle="popover"]').popover({
        placement: 'auto'
    });
    $('select[name="location_id"]').select2();
    </script>
@endsection
