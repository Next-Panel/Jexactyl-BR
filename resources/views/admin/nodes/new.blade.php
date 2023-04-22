@extends('layouts.admin')

@section('title')
    Nodes &rarr; Novo
@endsection

@section('content-header')
    <h1>Novo Node<small>Criar um novo Node local ou remoto para os servidores a serem instalados.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administração</a></li>
        <li><a href="{{ route('admin.nodes') }}">Nodes</a></li>
        <li class="active">Novo</li>
    </ol>
@endsection

@section('content')
<form action="{{ route('admin.nodes.new') }}" method="POST">
    <div class="row">
        <div class="col-sm-6">
            <div class="box box-primary">
                <div class="box-header with-border">
                    <h3 class="box-title">Detalhes Básicos</h3>
                </div>
                <div class="box-body">
                    <div class="form-group">
                        <label for="pName" class="form-label">Nome</label>
                        <input type="text" name="name" id="pName" class="form-control" value="{{ old('name') }}"/>
                        <p class="text-muted small">Limites de caracteres: <code>a-zA-Z0-9_.-</code> e <code>[Espaço]</code> (min 1, max 100 caracteres).</p>
                    </div>
                    <div class="form-group">
                        <label for="pDescription" class="form-label">Descrição</label>
                        <textarea name="description" id="pDescription" rows="4" class="form-control">{{ old('description') }}</textarea>
                    </div>
                    <div class="form-group">
                        <label for="pLocationId" class="form-label">Localização</label>
                        <select name="location_id" id="pLocationId">
                            @foreach($locations as $location)
                                <option value="{{ $location->id }}" {{ $location->id != old('location_id') ?: 'selected' }}>{{ $location->short }}</option>
                            @endforeach
                        </select>
                    </div>
                    <div class="form-group">
                        <label class="form-label">Visibilidade do Node</label>
                        <div>
                            <div class="radio radio-success radio-inline">

                                <input type="radio" id="pPublicTrue" value="1" name="public" checked>
                                <label for="pPublicTrue"> Público </label>
                            </div>
                            <div class="radio radio-danger radio-inline">
                                <input type="radio" id="pPublicFalse" value="0" name="public">
                                <label for="pPublicFalse"> Privado </label>
                            </div>
                        </div>
                        <p class="text-muted small">Ao definir um Node como<code>privado</code>, você estará negando a capacidade de implantar automaticamente nesse node.
                    </div>
                    <div class="form-group">
                        <label for="pFQDN" class="form-label">FQDN Wings</label>
                        <input type="text" name="fqdn" id="pFQDN" class="form-control" value="{{ old('fqdn') }}"/>
                        <p class="text-muted small">Insira o nome de domínio (por exemplo,<code> node.example.com</code>) a ser usado para se conectar ao daemon. Um endereço IP pode ser usado <em>somente</em> se você não estiver usando SSL para esse node.</p>
                    </div>
                    <div class="form-group">
                        <label for="daemonSFTPIP" class="control-label">FQDN SFTP (Opcional)</label>
                        <div>
                        <input type="text" name="daemonSFTPIP" class="form-control" id="pDaemonSFTPIP"/>
                        </div>
                        <p class="text-muted"><small>Insira o nome do domínio SFTP ou ip (por exemplo, <code>sftp.example.com</code> ou <code>123.456.789.123</code>) a ser usado para se conectar ao sftp do daemon.Ao utilizar um SFTP separado, é possível configurar um servidor Wings atrás do Proxy do Cloudflare. Isso pode melhorar a segurança e a eficiência do servidor.
                            </small></p>
                    </div>
                    <div class="form-group">
                        <label class="form-label">Comunicar por SSL</label>
                        <div>
                            <div class="radio radio-success radio-inline">
                                <input type="radio" id="pSSLTrue" value="https" name="scheme" checked>
                                <label for="pSSLTrue"> Usar conexão SSL</label>
                            </div>
                            <div class="radio radio-danger radio-inline">
                                <input type="radio" id="pSSLFalse" value="http" name="scheme" @if(request()->isSecure()) disabled @endif>
                                <label for="pSSLFalse"> Usar conexão HTTP</label>
                            </div>
                        </div>
                        @if(request()->isSecure())
                            <p class="text-danger small">Seu Painel está configurado no momento para usar uma conexão segura. Para que os navegadores se conectem ao seu node, ele <strong>deve</strong> usar uma conexão SSL.</p>
                        @else
                            <p class="text-muted small">Na maioria dos casos, você deve optar por usar uma conexão SSL. Se estiver usando um endereço IP ou se você não deseja usar SSL, selecione uma conexão HTTP.</p>
                        @endif
                    </div>
                    <div class="form-group">
                        <label class="form-label">Serviços CDN(Proxy)</label>
                        <div>
                            <div class="radio radio-success radio-inline">
                                <input type="radio" id="pProxyFalse" value="0" name="behind_proxy" checked>
                                <label for="pProxyFalse"> Não usar Proxy </label>
                            </div>
                            <div class="radio radio-info radio-inline">
                                <input type="radio" id="pProxyTrue" value="1" name="behind_proxy">
                                <label for="pProxyTrue"> Usar proxy </label>
                            </div>
                        </div>
                        <p class="text-muted small">Se você estiver usando serviços CDN que estão com proxy ativados coloque <code>"Usar Proxy"</code>,Caso contrario deixe <code>"Não usar Proxy"</code>.No CloudFlare isso pode não ser necessario.</p>
                    </div>
                </div>
            </div>
        </div>
        <div class="col-sm-6">
            <div class="box box-primary">
                <div class="box-header with-border">
                    <h3 class="box-title">Configurações</h3>
                </div>
                <div class="box-body">
                    <div class="row">
                        <div class="form-group col-md-6">
                            <label for="pDaemonBase" class="form-label">Diretório de arquivos do Servidor Daemon </label>
                            <input type="text" name="daemonBase" id="pDaemonBase" class="form-control" value="/var/lib/pterodactyl/volumes" />
                            <p class="text-muted small">Insira o diretório onde os arquivos do servidor devem ser armazenados. <strong>Se utilizar a OVH deve verificar o seu esquema de partição. Talvez seja necessário usar <code>/home/daemon-data</code> para ter espaço suficiente.</strong></p>
                        </div>
                        <div class="form-group col-md-6">
                            <label for="pMemory" class="form-label">Memoria Total </label>
                            <div class="input-group">
                                <input type="text" name="memory" data-multiplicator="true" class="form-control" id="pMemory" value="{{ old('memory') }}"/>
                                <span class="input-group-addon">MiB</span>
                            </div>
                        </div>
                        <div class="form-group col-md-6">
                            <label for="pMemoryOverallocate" class="form-label">Sobre-alocação de memória</label>
                            <div class="input-group">
                                <input type="text" name="memory_overallocate" class="form-control" id="pMemoryOverallocate" value="{{ old('memory_overallocate') }}"/>
                                <span class="input-group-addon">%</span>
                            </div>
                        </div>
                        <div class="col-md-12">
                            <p class="text-muted small">Insira a quantidade total de memória disponível para novos servidores. Se você quiser permitir a superalocação de memória, insira a porcentagem que deseja permitir. Para desativar a verificação de superalocação, insira <code>-1</code> no campo. Digitar <code>0</code> evitará a criação de novos servidores se ultrapassar o limite do Node.</p>
                        </div>
                    </div>
                    <div class="row">
                        <div class="form-group col-md-6">
                            <label for="pDisk" class="form-label">Espaço em disco Total</label>
                            <div class="input-group">
                                <input type="text" name="disk" data-multiplicator="true" class="form-control" id="pDisk" value="{{ old('disk') }}"/>
                                <span class="input-group-addon">MiB</span>
                            </div>
                        </div>
                        <div class="form-group col-md-6">
                            <label for="pDiskOverallocate" class="form-label">Super-alocação de disco</label>
                            <div class="input-group">
                                <input type="text" name="disk_overallocate" class="form-control" id="pDiskOverallocate" value="{{ old('disk_overallocate') }}"/>
                                <span class="input-group-addon">%</span>
                            </div>
                        </div>
                        <div class="col-md-12">
                            <p class="text-muted small">Insira a quantidade total de espaço em disco disponível para novos servidores. Se você quiser permitir a superalocação de espaço em disco, insira a porcentagem que deseja permitir. Para desativar a verificação de superalocação, insira <code>-1</code> no campo. Digitar <code>0</code> evitará a criação de novos servidores se ultrapassar o limite do Node.</p>
                        </div>
                    </div>
                        <div class="row">
                            <div class="form-group col-md-6">
                                <label for="daemonListen" class="control-label"><span class="label label-warning"><i class="fa fa-power-off"></i></span>Porta do Daemon</label>
                                    <div>
                                        <input type="text" name="daemonListen" class="form-control" id="pDaemonListen" value="8080" />
                                    </div>
                                </div>
                            <div class="form-group col-md-6">
                                <label for="daemonSFTP" class="control-label"><span class="label label-warning"><i class="fa fa-power-off"></i></span>Porta SFTP do Daemon</label>
                                    <div>
                                        <input type="text" name="daemonSFTP" class="form-control" id="pDaemonSFTP" value="2022" />
                                </div>
                        </div>
                    <div class="col-md-12">
                            <p class="text-muted small">O daemon executa seu próprio contêiner de gerenciamento SFTP e não utiliza o processo SSHd no servidor físico principal. <Strong>Não utilize a mesma porta que você designou para o processo SSH do seu servidor físico.</strong> Se você estiver executando o daemon atrás do CloudFlare&reg; você deve configurar o porta daemon para <code>8443</code> para permitir a proxy de websocket sobre SSL.</p>
                        </div>
                </div>
                <div class="box-footer">
                    {!! csrf_field() !!}
                    <button type="submit" class="btn btn-success pull-right">Criar Node</button>
                </div>
            </div>
        </div>
    </div>
</form>
@endsection

@section('footer-scripts')
    @parent
    <script>
        $('#pLocationId').select2();
    </script>
@endsection
